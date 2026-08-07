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
    sort_by: str  # "Frequency" or "Gap Size"

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
            "https://github.com/JunghunLeePhD/jumpchamp/releases/download/v1.0.0/primes.parquet"
        )
    )

# ============================================================================
# 2. Asset Ingestion Layer
# ============================================================================

def ensure_dataset_exists(config: AppConfig) -> None:
    """Validates existence and size of primes.parquet; downloads from release asset if missing/corrupt."""
    file_path = config.parquet_file
    
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
    conn = duckdb.connect()
    conn.sql("SET max_memory = '1GB';")
    conn.sql("SET threads = 2;")
    return conn

@st.cache_data(show_spinner=False)
def fetch_dataset_metadata(_conn: duckdb.DuckDBPyConnection, file_path: str) -> DatasetMetadata:
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
    """Queries gap frequency distribution using DuckDB SIMD vector subtraction and row-group skipping."""
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

def process_gap_dataframe(df: pd.DataFrame, sort_by: str) -> pd.DataFrame:
    """Calculates percentages and applies selected sorting order."""
    df = df.copy()
    total_pairs = df['frequency'].sum()
    df['percentage'] = (df['frequency'] / total_pairs * 100).round(2)
    
    # Sort DataFrame based on user preference
    if sort_by == "Gap Size":
        df = df.sort_values(by="diff", ascending=True)
    else:
        df = df.sort_values(by="frequency", ascending=False)

    df['diff_label'] = df['diff'].astype(str)
    return df

# ============================================================================
# 4. View Components Layer (Streamlit UI)
# ============================================================================

def inject_sticky_navbar_css() -> None:
    """Injects CSS to pin the top horizontal control row as a sticky header."""
    st.markdown(
        """
        <style>
        /* Pin top control row to top of page during vertical scrolling */
        div[data-testid="stHorizontalBlock"] {
            position: sticky;
            top: 2.875rem;
            z-index: 999;
            background-color: var(--background-color, #0e1117);
            padding-top: 0.75rem;
            padding-bottom: 0.75rem;
            border-bottom: 1px solid rgba(128, 128, 128, 0.2);
        }
        </style>
        """,
        unsafe_allow_html=True
    )

def render_math_definitions() -> None:
    """Renders LaTeX mathematical explanations for gap size and step size."""
    with st.expander("📖 Mathematical Definitions & Notation", expanded=False):
        st.markdown(
            r"""
            Let $p_n$ denote the $n$-th prime number in sequence ($p_1 = 2, p_2 = 3, p_3 = 5, \dots$).

            * **Step Size ($k$):** The index offset between prime numbers in sequence ($k \ge 1$).
              * When $k = 1$, we evaluate adjacent prime gaps: $p_{n+1} - p_n$.
              * When $k = 2$, we evaluate primes separated by one intervening prime: $p_{n+2} - p_n$.
            * **Gap Size ($\Delta$):** The arithmetic difference computed as:
              $$\Delta_k(n) = p_{n+k} - p_n$$
            """
        )

def render_top_filter_bar(meta: DatasetMetadata) -> FilterParams:
    """Renders sticky top control bar: Step Size (k) -> Sort Order -> Top N -> Min Prime -> Max Prime -> Slider."""
    default_max = min(meta.max_prime, 1_000_000)

    # Initialize state variables
    if "min_prime_val" not in st.session_state:
        st.session_state.min_prime_val = meta.min_prime
    if "max_prime_val" not in st.session_state:
        st.session_state.max_prime_val = default_max
    if "slider_bounds" not in st.session_state:
        st.session_state.slider_bounds = (st.session_state.min_prime_val, st.session_state.max_prime_val)

    # Sync callbacks
    def sync_from_slider():
        st.session_state.min_prime_val = st.session_state.slider_bounds[0]
        st.session_state.max_prime_val = st.session_state.slider_bounds[1]

    def sync_from_numbers():
        min_v = max(meta.min_prime, min(st.session_state.min_prime_val, meta.max_prime - 1))
        max_v = min(meta.max_prime, max(st.session_state.max_prime_val, min_v + 1))
        st.session_state.min_prime_val = min_v
        st.session_state.max_prime_val = max_v
        st.session_state.slider_bounds = (min_v, max_v)

    col1, col2, col3, col4, col5, col6 = st.columns([1.2, 1, 1, 1.1, 1.1, 2.2])

    with col1:
        k = st.number_input(
            "Step Size (k)",
            min_value=1,
            max_value=20,
            value=2,
            help="Computes distance between primes across k steps."
        )

    with col2:
        sort_by = st.selectbox(
            "Sort Order",
            options=["Frequency", "Gap Size"],
            index=0,
            help="'Frequency' sorts descending by count; 'Gap Size' orders numerically along the X-axis."
        )

    with col3:
        top_n = st.number_input(
            "Top N Gaps",
            min_value=5,
            max_value=50,
            value=20,
            step=5
        )

    with col4:
        st.number_input(
            "Min Prime",
            min_value=meta.min_prime,
            max_value=meta.max_prime - 1,
            key="min_prime_val",
            on_change=sync_from_numbers,
            step=100_000
        )

    with col5:
        st.number_input(
            "Max Prime",
            min_value=meta.min_prime + 1,
            max_value=meta.max_prime,
            key="max_prime_val",
            on_change=sync_from_numbers,
            step=100_000
        )

    with col6:
        st.slider(
            "Prime Range Slider",
            min_value=meta.min_prime,
            max_value=meta.max_prime,
            key="slider_bounds",
            on_change=sync_from_slider
        )

    return FilterParams(
        k=int(k),
        min_prime=int(st.session_state.min_prime_val),
        max_prime=int(st.session_state.max_prime_val),
        top_n=int(top_n),
        sort_by=sort_by
    )

def render_gap_distribution_chart(df: pd.DataFrame) -> None:
    """Renders clean Plotly bar chart maintaining the DataFrame's sorted order."""
    fig = px.bar(
        df,
        x="diff_label",
        y="frequency",
        text="percentage",
        labels={"diff_label": "Gap Size", "frequency": "Frequency"},
        hover_data={"diff_label": True, "frequency": ":,", "percentage": ":.2f%"},
        color="frequency",
        color_continuous_scale="Viridis"
    )
    fig.update_traces(texttemplate='%{text}%', textposition='outside')
    fig.update_layout(
        xaxis_title="Gap Size",
        yaxis_title="Frequency Count",
        showlegend=False,
        height=420,
        margin=dict(l=10, r=10, t=10, b=10),
        font=dict(family="Inter, system-ui, sans-serif", size=13)
    )
    fig.update_xaxes(type='category', categoryorder='array', categoryarray=df['diff_label'])

    st.plotly_chart(fig, width="stretch")

def render_data_table(df: pd.DataFrame) -> None:
    """Renders tabular format data with explicit Rank indices."""
    display_df = df[['diff', 'frequency', 'percentage']].copy()
    display_df.insert(0, 'Rank', range(1, len(display_df) + 1))
    display_df.columns = ['Rank', 'Gap Size', 'Frequency', 'Percentage']
    display_df['Frequency'] = display_df['Frequency'].map('{:,}'.format)
    display_df['Percentage'] = display_df['Percentage'].map('{:.2f}%'.format)

    st.dataframe(display_df, hide_index=True, width="stretch")

# ============================================================================
# 5. Main Application Orchestrator
# ============================================================================

def main():
    st.set_page_config(page_title="Prime Gap Explorer", page_icon="🦀", layout="wide")

    # 1. Inject Sticky Navbar CSS
    inject_sticky_navbar_css()

    # 2. Config & Data File Verification
    config = load_config()
    ensure_dataset_exists(config)

    # 3. Connection & Metadata
    conn = get_db_connection()
    metadata = fetch_dataset_metadata(conn, config.parquet_file)

    # 4. Mathematical Definitions & Sticky Control Bar
    render_math_definitions()
    params = render_top_filter_bar(metadata)

    # 5. Data Fetch & Process
    with st.spinner("Executing DuckDB query..."):
        raw_df = query_prime_gaps(conn, config.parquet_file, params)

    if raw_df.empty:
        st.warning("No prime pairs found in the selected range for this step size k.")
        return

    df = process_gap_dataframe(raw_df, params.sort_by)

    # 6. Vertical Layout: Chart followed by Data Table
    render_gap_distribution_chart(df)
    render_data_table(df)

if __name__ == "__main__":
    main()