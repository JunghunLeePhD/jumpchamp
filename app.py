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
    release_url_template: str

@dataclass(frozen=True)
class FilterParams:
    gap_k: int
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


def load_config() -> AppConfig:
    """Loads default configuration for dataset file path template and remote release download URL."""
    return AppConfig(
        release_url_template=DEFAULT_RELEASE_URL_TEMPLATE,
    )

# ============================================================================
# 2. Asset Ingestion Layer
# ============================================================================

def ensure_dataset_exists(gap_k: int, config: AppConfig) -> str:
    """Validates existence of gaps{k}.parquet; downloads from release asset if missing/corrupt."""
    gaps_file = f"gaps{gap_k}.parquet"

    if os.path.exists(gaps_file) and os.path.getsize(gaps_file) > 100_000:
        return gaps_file

    release_url = config.release_url_template.format(k=gap_k)
    st.info(f"📦 {gap_k}-Step Gap Database (`{gaps_file}`) not found locally. Fetching remote storage (~2048 MB)...")
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
        urllib.request.urlretrieve(release_url, gaps_file, reporthook=_download_callback)
        progress_bar.empty()
        status_text.empty()
        st.success(f"✅ Download complete! Initializing database engine for {gap_k}-step gaps...")
        st.rerun()
    except Exception as e:
        progress_bar.empty()
        status_text.empty()
        if os.path.exists(gaps_file):
            os.remove(gaps_file)
        st.error(
            f"❌ Database (`{gaps_file}`) not found locally and could not be fetched from remote storage.\n\n"
            f"Please run `cargo run --release --bin build_gaps -- {gap_k}` to generate `{gaps_file}` locally.\n\n"
            f"Error details: {e}"
        )
        st.stop()
    return gaps_file

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
    meta = _conn.sql(f"SELECT 1, COUNT(*), COUNT(*), COUNT(DISTINCT deltak) FROM '{gaps_file}'").fetchone()
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
    """Queries k-step gap frequency distribution by prime index range [min_idx, max_idx].
    
    Zero windowing, zero subtractions: direct single-column row offset slice & GROUP BY.
    """
    offset = params.min_idx - 1
    limit_count = params.max_idx - params.min_idx + 1

    query = f"""
    WITH sliced AS (
        SELECT deltak FROM '{gaps_file}'
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
        /* Pin top control row / form to top of page during vertical scrolling */
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
    with st.expander("📖 Mathematical Definitions & Notation", expanded=False):
        st.markdown(
            f"""
            Let $p_n$ denote the $n$-th prime number in sequence ($p_1 = 2, p_2 = 3, p_3 = 5, \\dots$).

            * **{gap_k}-Step Gap ($\Delta_{{{subscript}}}$):** The arithmetic difference between primes separated by {intervening} intervening prime(s):
              $$\Delta_{{{subscript}}}(n) = p_{{n+{gap_k}}} - p_n$$
            * **Prime Index ($n$):** Filters are applied to prime sequence numbers $n$ to $m$ ($p_n$ to $p_m$).
            """
        )

def render_top_filter_bar(meta: DatasetMetadata, is_processing: bool = False) -> FilterParams:
    """Renders sticky top control bar without an Apply button, updating live on parameter changes."""
    default_max = min(meta.max_idx, 100_000_000)

    # Initialize / validate state variables
    if "min_idx_val" not in st.session_state:
        st.session_state.min_idx_val = meta.min_idx
    else:
        st.session_state.min_idx_val = max(meta.min_idx, min(st.session_state.min_idx_val, meta.max_idx - 1))

    if "max_idx_val" not in st.session_state:
        st.session_state.max_idx_val = default_max
    else:
        st.session_state.max_idx_val = min(meta.max_idx, max(st.session_state.max_idx_val, st.session_state.min_idx_val + 1))

    st.session_state.slider_bounds = (st.session_state.min_idx_val, st.session_state.max_idx_val)

    # Sync callbacks
    def sync_from_slider():
        st.session_state.min_idx_val = st.session_state.slider_bounds[0]
        st.session_state.max_idx_val = st.session_state.slider_bounds[1]

    def sync_from_numbers():
        min_v = max(meta.min_idx, min(st.session_state.min_idx_val, meta.max_idx - 1))
        max_v = min(meta.max_idx, max(st.session_state.max_idx_val, min_v + 1))
        st.session_state.min_idx_val = min_v
        st.session_state.max_idx_val = max_v
        st.session_state.slider_bounds = (min_v, max_v)

    col1, col2, col3, col4, col5, col6 = st.columns([0.9, 1.0, 0.9, 1.1, 1.1, 2.2])

    with col1:
        gap_k = st.selectbox(
            "Step Size (k)",
            options=[2, 3],
            index=0,
            key="gap_k_val",
            disabled=is_processing,
            help="Select k for k-step gap calculation: Δ_k(n) = p_{n+k} - p_n"
        )

    with col2:
        sort_by = st.selectbox(
            "Sort Order",
            options=["Frequency", "Gap Size"],
            index=0,
            disabled=is_processing,
            help="'Frequency' sorts descending by count; 'Gap Size' orders numerically along the X-axis."
        )

    with col3:
        top_n = st.number_input(
            "Top N Gaps",
            min_value=5,
            max_value=max(5, meta.unique_gaps_count),
            value=min(20, meta.unique_gaps_count),
            step=5,
            disabled=is_processing,
            help=f"Select top N gaps to display (Total unique gap sizes in dataset: {meta.unique_gaps_count})"
        )

    with col4:
        st.number_input(
            "Min Index (n)",
            min_value=meta.min_idx,
            max_value=meta.max_idx - 1,
            key="min_idx_val",
            on_change=sync_from_numbers,
            step=100_000,
            disabled=is_processing
        )

    with col5:
        st.number_input(
            "Max Index (m)",
            min_value=meta.min_idx + 1,
            max_value=meta.max_idx,
            key="max_idx_val",
            on_change=sync_from_numbers,
            step=100_000,
            disabled=is_processing
        )

    with col6:
        st.slider(
            "Prime Index Range (n to m)",
            min_value=meta.min_idx,
            max_value=meta.max_idx,
            key="slider_bounds",
            on_change=sync_from_slider,
            disabled=is_processing
        )

    return FilterParams(
        gap_k=int(gap_k),
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

    # Calculate exact height to fit all rows without internal vertical scrolling
    dynamic_height = (len(display_df) + 1) * 36 + 3
    st.dataframe(display_df, hide_index=True, width="stretch", height=dynamic_height)

# ============================================================================
# 5. Main Application Orchestrator
# ============================================================================

def main():
    gap_k = st.session_state.get("gap_k_val", 2)
    st.set_page_config(page_title=f"{gap_k}-Step Prime Gap Explorer", page_icon="🦀", layout="wide")

    # 1. Inject Sticky Navbar CSS
    inject_sticky_navbar_css()

    # 2. Config & Data File Verification
    config = load_config()
    gaps_file = ensure_dataset_exists(gap_k, config)

    # 3. Connection & Metadata
    conn = get_db_connection()
    metadata = fetch_dataset_metadata(conn, gaps_file)

    # 4. Mathematical Definitions & Sticky Control Bar
    render_math_definitions(gap_k)
    
    is_processing = st.session_state.get("is_processing", False)
    params = render_top_filter_bar(metadata, is_processing=is_processing)

    # Lock processing while fetching and rendering
    st.session_state["is_processing"] = True
    try:
        with st.spinner("Executing DuckDB query & rendering visualisations..."):
            raw_df = query_prime_gaps(conn, gaps_file, params)

            if raw_df.empty:
                st.warning("No prime pairs found in the selected index range.")
                return

            df = process_gap_dataframe(raw_df, params.sort_by)

            # 6. Vertical Layout: Chart followed by Data Table
            render_gap_distribution_chart(df, params.gap_k)
            render_data_table(df, params.gap_k)
    finally:
        st.session_state["is_processing"] = False

if __name__ == "__main__":
    main()