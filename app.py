import os
import urllib.request
from dataclasses import dataclass
from typing import NamedTuple

import duckdb
import pandas as pd
import plotly.express as px
import streamlit as st
from dotenv import load_dotenv

# ============================================================================
# 1. Domain Types & Configuration Layer
# ============================================================================

@dataclass(frozen=True)
class AppConfig:
    parquet_file: str
    release_url: str

class FilterParams(NamedTuple):
    k: int
    min_prime: int
    max_prime: int
    top_n: int

class DatasetMetadata(NamedTuple):
    min_prime: int
    max_prime: int
    total_count: int


def load_config() -> AppConfig:
    """Loads configuration safely from OS environment, Streamlit secrets, or defaults."""
    load_dotenv()
    
    def _get_val(key: str, default: str) -> str:
        if os.getenv(key):
            return os.getenv(key)
        try:
            if key in st.secrets:
                return st.secrets[key]
        except Exception:
            pass
        return default

    return AppConfig(
        parquet_file=_get_val("PARQUET_FILE_PATH", "primes.parquet"),
        release_url=_get_val(
            "RELEASE_URL",
            "https://github.com/JunghunLeePhD/primes/releases/download/v1.0.0/primes.parquet"
        )
    )

# ============================================================================
# 2. Asset Ingestion & Storage Layer
# ============================================================================

def ensure_dataset_exists(config: AppConfig) -> None:
    """Validates existence and size of primes.parquet; downloads from release asset if missing/corrupt."""
    file_path = config.parquet_file
    
    # Remove empty or corrupt files (< 1MB)
    if os.path.exists(file_path):
        if os.path.getsize(file_path) < 1_000_000:
            os.remove(file_path)
        else:
            return

    st.info(f"📦 Dataset (`{file_path}`) not found locally. Fetching remote storage...")
    progress_bar = st.progress(0.0)
    status_text = st.empty()

    def _download_callback(block_num: int, block_size: int, total_size: int):
        downloaded = block_num * block_size
        if total_size > 0:
            percent = min(1.0, downloaded / total_size)
            progress_bar.progress(percent)
            status_text.text(
                f"Downloading: {downloaded / (1024*1024):.1f} MB / {total_size / (1024*1024):.1f} MB ({int(percent * 100)}%)"
            )

    try:
        urllib.request.urlretrieve(config.release_url, file_path, reporthook=_download_callback)
        progress_bar.empty()
        status_text.empty()
        st.success("✅ Download complete! Initializing database engine...")
        st.rerun()
    except Exception as e:
        progress_bar.empty()
        status_text.empty()
        if os.path.exists(file_path):
            os.remove(file_path)
        st.error(f"❌ Download failed from URL:\n`{config.release_url}`\n\nError: {e}")
        st.stop()

# ============================================================================
# 3. Database Engine & Query Layer (DuckDB)
# ============================================================================

@st.cache_resource
def get_db_connection() -> duckdb.DuckDBPyConnection:
    """Initializes a shared DuckDB connection capped to safe memory limits."""
    conn = duckdb.connect()
    conn.sql("SET max_memory = '1GB';")
    conn.sql("SET threads = 2;")
    return conn

@st.cache_data(show_spinner=False)
def fetch_dataset_metadata(_conn: duckdb.DuckDBPyConnection, file_path: str) -> DatasetMetadata:
    """Queries min/max prime bounds and total count in O(1) time."""
    meta = _conn.sql(f"""
        SELECT MIN(prime), MAX(prime), COUNT(*) 
        FROM '{file_path}'
    """).fetchone()
    return DatasetMetadata(min_prime=int(meta[0]), max_prime=int(meta[1]), total_count=int(meta[2]))

@st.cache_data(show_spinner=False)
def query_prime_gaps(
    _conn: duckdb.DuckDBPyConnection, 
    file_path: str, 
    params: FilterParams
) -> pd.DataFrame:
    """Executes C++ window function to calculate gap distribution for step size k."""
    query = f"""
    WITH interval_primes AS (
        SELECT prime FROM '{file_path}'
        WHERE prime BETWEEN {params.min_prime} AND {params.max_prime}
    ),
    gaps AS (
        SELECT LEAD(prime, {params.k}) OVER (ORDER BY prime) - prime AS diff
        FROM interval_primes
    )
    SELECT diff, COUNT(*) AS frequency
    FROM gaps
    WHERE diff IS NOT NULL
    GROUP BY diff
    ORDER BY frequency DESC
    LIMIT {params.top_n};
    """
    return _conn.sql(query).df()

def process_gap_dataframe(df: pd.DataFrame) -> pd.DataFrame:
    """Transforms raw SQL query results into formatted display vectors."""
    df = df.copy()
    total_pairs = df['frequency'].sum()
    df['percentage'] = (df['frequency'] / total_pairs * 100).round(2)
    df['diff_label'] = df['diff'].astype(str)
    return df

# ============================================================================
# 4. View Components Layer (Streamlit UI)
# ============================================================================

def render_sidebar(meta: DatasetMetadata) -> FilterParams:
    """Renders filter controls and returns strongly typed user inputs."""
    st.sidebar.header("⚙️ Filter Parameters")
    st.sidebar.metric("Total Primes in DB", f"{meta.total_count:,}")
    st.sidebar.metric("Prime Range", f"{meta.min_prime:,} to {meta.max_prime:,}")
    st.sidebar.markdown("---")

    k = st.sidebar.number_input(
        "Step Size (k)", min_value=1, max_value=20, value=2,
        help="Computes difference between p_{n+k} and p_n."
    )
    
    default_max = min(meta.max_prime, 1_000_000)
    st.sidebar.subheader("Select Prime Interval Bounds")
    
    min_p = st.sidebar.number_input(
        "Min Prime (A)", min_value=meta.min_prime, max_value=meta.max_prime - 1, value=meta.min_prime
    )
    max_p = st.sidebar.number_input(
        "Max Prime (B)", min_value=min_p + 1, max_value=meta.max_prime, value=default_max
    )
    top_n = st.sidebar.slider("Top N Gaps to Show", min_value=5, max_value=50, value=20)

    return FilterParams(k=int(k), min_prime=int(min_p), max_prime=int(max_p), top_n=int(top_n))

def render_kpi_cards(k: int, df: pd.DataFrame) -> None:
    """Displays top-level metric highlights."""
    most_frequent_gap = int(df.iloc[0]['diff'])
    highest_pct = df.iloc[0]['percentage']

    col1, col2, col3 = st.columns(3)
    col1.metric("Selected Step Size", f"k = {k}")
    col2.metric("Most Frequent Gap", f"Δ = {most_frequent_gap}")
    col3.metric("Top Gap Percentage", f"{highest_pct}%")

def render_gap_distribution_chart(df: pd.DataFrame, k: int) -> None:
    """Renders Plotly interactive frequency distribution bar chart."""
    st.subheader(f"Frequency Distribution for $p_{{n+{k}}} - p_n$")
    fig = px.bar(
        df,
        x="diff_label", y="frequency", text="percentage",
        labels={"diff_label": f"Difference ($p_{{n+{k}}} - p_n$)", "frequency": "Frequency"},
        hover_data={"diff_label": True, "frequency": ":,", "percentage": ":.2f%"},
        color="frequency", color_continuous_scale="Viridis"
    )
    fig.update_traces(texttemplate='%{text}%', textposition='outside')
    fig.update_layout(
        xaxis_title=f"Gap Size ($p_{{n+{k}}} - p_n$)",
        yaxis_title="Count",
        xaxis={'type': 'category'},
        showlegend=False,
        height=500
    )
    st.plotly_chart(fig, width="stretch")

def render_data_table_and_export(df: pd.DataFrame, params: FilterParams) -> None:
    """Displays formatted tabular data and CSV export capability."""
    st.subheader("📊 Data Table")
    
    display_df = df[['diff', 'frequency', 'percentage']].copy()
    display_df.columns = ['Gap (Diff)', 'Frequency', 'Percentage']
    display_df['Frequency'] = display_df['Frequency'].map('{:,}'.format)
    display_df['Percentage'] = display_df['Percentage'].map('{:.2f}%'.format)
    
    st.dataframe(display_df, hide_index=True, width="stretch")

    csv_data = display_df.to_csv(index=False).encode('utf-8')
    st.download_button(
        label="📥 Download Table as CSV",
        data=csv_data,
        file_name=f"prime_gaps_k{params.k}_interval_{params.min_prime}_{params.max_prime}.csv",
        mime="text/csv"
    )

# ============================================================================
# 5. Main Application Orchestrator
# ============================================================================

def main():
    st.set_page_config(page_title="Prime Gap Explorer", page_icon="🦀", layout="wide")
    st.title("🦀 Prime Gap Distribution Explorer")
    st.markdown("""
    Analyze the frequency of gaps between prime numbers ($p_{n+k} - p_n$) across arbitrary numerical bounds.
    This app streams directly from a compressed Parquet database using DuckDB.
    """)

    # 1. Config & Data Source
    config = load_config()
    ensure_dataset_exists(config)

    # 2. Connection & Metadata
    conn = get_db_connection()
    metadata = fetch_dataset_metadata(conn, config.parquet_file)

    # 3. Sidebar View & User Input
    params = render_sidebar(metadata)

    # 4. Data Fetch & Process (Cached execution)
    with st.spinner("Executing DuckDB query..."):
        raw_df = query_prime_gaps(conn, config.parquet_file, params)

    if raw_df.empty:
        st.warning("No prime pairs found in the selected range for this step size k.")
        return

    df = process_gap_dataframe(raw_df)

    # 5. View Rendering
    render_kpi_cards(params.k, df)
    st.markdown("---")

    left_col, right_col = st.columns([2, 1])
    with left_col:
        render_gap_distribution_chart(df, params.k)
    with right_col:
        render_data_table_and_export(df, params)

if __name__ == "__main__":
    main()