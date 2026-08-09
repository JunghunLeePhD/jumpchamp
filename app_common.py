import os
import threading
import urllib.request
from dataclasses import dataclass

import duckdb
import pandas as pd
import plotly.express as px
import streamlit as st

_db_lock = threading.Lock()

# ============================================================================
# 1. Domain Types & Configuration Layer
# ============================================================================

@dataclass(frozen=True)
class AppConfig:
    gaps_file: str
    release_url: str

@dataclass(frozen=True)
class FilterParams:
    min_idx: int
    max_idx: int
    top_n: int
    sort_by: str  # "Frequency" or "Gap Size"

@dataclass(frozen=True)
class DatasetMetadata:
    min_idx: int
    max_idx: int
    total_count: int
    unique_gaps_count: int


DEFAULT_RELEASE_URL_TEMPLATE = (
    "https://github.com/JunghunLeePhD/jumpchamp/releases/download/v1.0.0/gaps{k}.parquet"
)


def load_config(gap_k: int) -> AppConfig:
    """Loads default configuration for k-step gap dataset file path and remote release download URL."""
    gaps_file = f"gaps{gap_k}.parquet"
    release_url = DEFAULT_RELEASE_URL_TEMPLATE.format(k=gap_k)
    return AppConfig(
        gaps_file=gaps_file,
        release_url=release_url,
    )

# ============================================================================
# 2. Asset Ingestion Layer
# ============================================================================

def ensure_dataset_exists(gap_k: int, config: AppConfig) -> None:
    """Validates existence of gaps{k}.parquet; downloads from release asset if missing/corrupt."""
    gaps_path = config.gaps_file

    if os.path.exists(gaps_path) and os.path.getsize(gaps_path) > 100_000:
        return

    st.info(f"📦 {gap_k}-Step Gap Database (`{gaps_path}`) not found locally. Fetching remote storage...")
    progress_bar = st.progress(0.0)
    status_text = st.empty()

    temp_path = f".{gaps_path}.tmp"

    def _download_callback(block_num: int, block_size: int, total_size: int):
        downloaded = block_num * block_size
        if total_size > 0:
            percent = min(1.0, downloaded / total_size)
            progress_bar.progress(percent)
            status_text.text(
                f"Downloading: {downloaded / (1024*1024):.1f} MB / {total_size / (1024*1024):.1f} MB ({int(percent * 100)}%)"
            )

    try:
        req = urllib.request.Request(config.release_url, headers={"User-Agent": "Mozilla/5.0"})
        with urllib.request.urlopen(req) as response, open(temp_path, "wb") as out_file:
            total_size = int(response.headers.get("Content-Length", 0))
            downloaded = 0
            block_size = 1024 * 1024  # 1MB chunks
            block_num = 0
            while True:
                chunk = response.read(block_size)
                if not chunk:
                    break
                out_file.write(chunk)
                downloaded += len(chunk)
                block_num += 1
                _download_callback(block_num, block_size, total_size)

        progress_bar.empty()
        status_text.empty()

        if os.path.exists(temp_path) and os.path.getsize(temp_path) > 100_000:
            os.replace(temp_path, gaps_path)
            st.success("✅ Download complete! Initializing database engine...")
            st.rerun()
        else:
            raise RuntimeError("Downloaded file is empty or corrupted.")
    except Exception as e:
        progress_bar.empty()
        status_text.empty()
        if os.path.exists(temp_path):
            os.remove(temp_path)
        if os.path.exists(gaps_path) and os.path.getsize(gaps_path) <= 100_000:
            os.remove(gaps_path)
        st.error(
            f"❌ Database (`{gaps_path}`) not found locally and could not be fetched from remote storage.\n\n"
            f"Please run `cargo run --release --bin build_gaps -- {gap_k}` to generate `{gaps_path}` locally.\n\n"
            f"Error details: {e}"
        )
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
def fetch_dataset_metadata(_conn: duckdb.DuckDBPyConnection, gaps_file: str) -> DatasetMetadata:
    if not os.path.exists(gaps_file) or os.path.getsize(gaps_file) <= 100_000:
        st.error(f"❌ Dataset file `{gaps_file}` is missing or corrupted.")
        st.stop()
    escaped_path = gaps_file.replace("'", "''")
    meta = _conn.sql(f"SELECT 1, COUNT(*), COUNT(*), COUNT(DISTINCT deltak) FROM '{escaped_path}'").fetchone()
    return DatasetMetadata(
        min_idx=int(meta[0]),
        max_idx=int(meta[1]),
        total_count=int(meta[2]),
        unique_gaps_count=int(meta[3]),
    )

@st.cache_data(show_spinner=False)
def query_prime_gaps(
    _conn: duckdb.DuckDBPyConnection,
    gaps_file: str,
    params: FilterParams,
) -> pd.DataFrame:
    """Queries k-step gap frequency distribution by prime index range [min_idx, max_idx]."""
    if not os.path.exists(gaps_file) or os.path.getsize(gaps_file) <= 100_000:
        st.error(f"❌ Dataset file `{gaps_file}` is missing or corrupted.")
        st.stop()

    offset = params.min_idx - 1
    limit_count = params.max_idx - params.min_idx + 1
    escaped_path = gaps_file.replace("'", "''")

    query = f"""
    WITH sliced AS (
        SELECT deltak FROM '{escaped_path}'
        LIMIT {limit_count} OFFSET {offset}
    )
    SELECT deltak AS diff, COUNT(*) AS frequency
    FROM sliced
    GROUP BY deltak
    ORDER BY frequency DESC
    LIMIT {params.top_n};
    """

    with _db_lock:
        return _conn.sql(query).df()

def process_gap_dataframe(df: pd.DataFrame, sort_by: str) -> pd.DataFrame:
    """Calculates percentages and applies selected sorting order."""
    df = df.copy()
    total_pairs = df['frequency'].sum()
    df['percentage'] = (df['frequency'] / total_pairs * 100).round(2)
    
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
        div[data-testid="stHorizontalBlock"], div[data-testid="stForm"] {
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

def render_math_definitions(gap_k: int = 2) -> None:
    """Renders LaTeX mathematical explanations for k-step prime gap size."""
    subscript = str(gap_k)
    intervening = gap_k - 1
    intervening_str = "one intervening prime" if intervening == 1 else f"{intervening} intervening primes"

    with st.expander("📖 Mathematical Definitions & Notation", expanded=False):
        st.markdown(
            f"""
            Let $p_n$ denote the $n$-th prime number in sequence ($p_1 = 2, p_2 = 3, p_3 = 5, \\dots$).

            * **{gap_k}-Step Gap ($\Delta_{{{subscript}}}$):** The arithmetic difference between primes separated by {intervening_str}:
              $$\Delta_{{{subscript}}}(n) = p_{{n+{gap_k}}} - p_n$$
            * **Prime Index ($n$):** Filters are applied to prime sequence numbers $n$ to $m$ ($p_n$ to $p_m$).
            """
        )

def render_top_filter_bar(meta: DatasetMetadata, is_processing: bool = False) -> FilterParams:
    """Renders sticky top control bar without an Apply button, updating live on parameter changes."""
    default_max = min(meta.max_idx, 100_000_000)

    if "min_idx_val" not in st.session_state:
        st.session_state.min_idx_val = meta.min_idx
    if "max_idx_val" not in st.session_state:
        st.session_state.max_idx_val = default_max
    if "slider_bounds" not in st.session_state:
        st.session_state.slider_bounds = (st.session_state.min_idx_val, st.session_state.max_idx_val)

    def sync_from_slider():
        st.session_state.min_idx_val = st.session_state.slider_bounds[0]
        st.session_state.max_idx_val = st.session_state.slider_bounds[1]

    def sync_from_numbers():
        min_v = max(meta.min_idx, min(st.session_state.min_idx_val, meta.max_idx - 1))
        max_v = min(meta.max_idx, max(st.session_state.max_idx_val, min_v + 1))
        st.session_state.min_idx_val = min_v
        st.session_state.max_idx_val = max_v
        st.session_state.slider_bounds = (min_v, max_v)

    col1, col2, col3, col4, col5 = st.columns([1, 1, 1.1, 1.1, 2.2])

    with col1:
        sort_by = st.selectbox(
            "Sort Order",
            options=["Frequency", "Gap Size"],
            index=0,
            disabled=is_processing,
            help="'Frequency' sorts descending by count; 'Gap Size' orders numerically along the X-axis."
        )

    with col2:
        top_n = st.number_input(
            "Top N Gaps",
            min_value=5,
            max_value=max(5, meta.unique_gaps_count),
            value=min(20, meta.unique_gaps_count),
            step=5,
            disabled=is_processing,
            help=f"Select top N gaps to display (Total unique gap sizes in dataset: {meta.unique_gaps_count})"
        )

    with col3:
        st.number_input(
            "Min Index (n)",
            min_value=meta.min_idx,
            max_value=meta.max_idx - 1,
            key="min_idx_val",
            on_change=sync_from_numbers,
            step=100_000,
            disabled=is_processing
        )

    with col4:
        st.number_input(
            "Max Index (m)",
            min_value=meta.min_idx + 1,
            max_value=meta.max_idx,
            key="max_idx_val",
            on_change=sync_from_numbers,
            step=100_000,
            disabled=is_processing
        )

    with col5:
        st.slider(
            "Prime Index Range (n to m)",
            min_value=meta.min_idx,
            max_value=meta.max_idx,
            key="slider_bounds",
            on_change=sync_from_slider,
            disabled=is_processing
        )

    return FilterParams(
        min_idx=int(st.session_state.min_idx_val),
        max_idx=int(st.session_state.max_idx_val),
        top_n=int(top_n),
        sort_by=sort_by
    )

def render_gap_distribution_chart(df: pd.DataFrame, gap_k: int = 2) -> None:
    """Renders clean Plotly bar chart maintaining the DataFrame's sorted order."""
    subscript_map = {2: "₂", 3: "₃"}
    sub_char = subscript_map.get(gap_k, f"_{gap_k}")
    gap_label = f"{gap_k}-Step Gap Size (Δ{sub_char})"

    fig = px.bar(
        df,
        x="diff_label",
        y="frequency",
        text="percentage",
        labels={"diff_label": gap_label, "frequency": "Frequency"},
        hover_data={"diff_label": True, "frequency": ":,", "percentage": ":.2f%"},
        color="frequency",
        color_continuous_scale="Viridis"
    )
    fig.update_traces(texttemplate='%{text}%', textposition='outside')
    fig.update_layout(
        xaxis_title=gap_label,
        yaxis_title="Frequency Count",
        showlegend=False,
        height=420,
        margin=dict(l=10, r=10, t=10, b=10),
        font=dict(family="Inter, system-ui, sans-serif", size=13)
    )
    fig.update_xaxes(type='category', categoryorder='array', categoryarray=df['diff_label'])

    st.plotly_chart(fig, width="stretch")

def render_data_table(df: pd.DataFrame, gap_k: int = 2) -> None:
    """Renders tabular format data fitting all rows completely without internal scrollbars."""
    subscript_map = {2: "₂", 3: "₃"}
    sub_char = subscript_map.get(gap_k, f"_{gap_k}")
    gap_label = f"{gap_k}-Step Gap Size (Δ{sub_char})"

    display_df = df[['diff', 'frequency', 'percentage']].copy()
    display_df.insert(0, 'Rank', range(1, len(display_df) + 1))
    display_df.columns = ['Rank', gap_label, 'Frequency', 'Percentage']
    display_df['Frequency'] = display_df['Frequency'].map('{:,}'.format)
    display_df['Percentage'] = display_df['Percentage'].map('{:.2f}%'.format)

    dynamic_height = (len(display_df) + 1) * 36 + 3
    st.dataframe(display_df, hide_index=True, width="stretch", height=dynamic_height)

# ============================================================================
# 5. Main Application Orchestrator
# ============================================================================

def run_app(gap_k: int) -> None:
    """Runs the Streamlit dashboard for a given k-step gap size."""
    st.set_page_config(page_title=f"{gap_k}-Step Prime Gap Explorer", page_icon="🦀", layout="wide")

    inject_sticky_navbar_css()

    config = load_config(gap_k)
    ensure_dataset_exists(gap_k, config)

    conn = get_db_connection()
    metadata = fetch_dataset_metadata(conn, config.gaps_file)

    render_math_definitions(gap_k)

    is_processing = st.session_state.get("is_processing", False)
    params = render_top_filter_bar(metadata, is_processing=is_processing)

    st.session_state["is_processing"] = True
    try:
        with st.spinner("Executing DuckDB query & rendering visualisations..."):
            raw_df = query_prime_gaps(conn, config.gaps_file, params)

            if raw_df.empty:
                st.warning("No prime pairs found in the selected index range.")
                return

            df = process_gap_dataframe(raw_df, params.sort_by)

            render_gap_distribution_chart(df, gap_k)
            render_data_table(df, gap_k)
    finally:
        st.session_state["is_processing"] = False
