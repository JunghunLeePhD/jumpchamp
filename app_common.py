import concurrent.futures
import os
import threading
import time
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
    """Validates existence of gaps{k}.parquet; downloads from release asset with resumable Range support if missing."""
    gaps_path = config.gaps_file

    if os.path.exists(gaps_path) and os.path.getsize(gaps_path) > 100_000_000:
        return

    temp_path = f".{gaps_path}.tmp"
    existing_bytes = os.path.getsize(temp_path) if os.path.exists(temp_path) else 0

    st.info(f"📦 {gap_k}-Step Gap Database (`{gaps_path}`) missing or incomplete. Downloading remote release dataset (~660 MB)...")
    progress_bar = st.progress(0.0)
    status_text = st.empty()

    def _update_progress(current: int, total: int):
        if total > 0:
            percent = min(1.0, current / total)
            progress_bar.progress(percent)
            status_text.text(
                f"Downloading `{gaps_path}`: {current / (1024*1024):.1f} MB / {total / (1024*1024):.1f} MB ({int(percent * 100)}%)"
            )

    try:
        download_url = resolve_direct_url(config.release_url)
        headers = {"User-Agent": "Mozilla/5.0"}
        if existing_bytes > 0:
            headers["Range"] = f"bytes={existing_bytes}-"

        req = urllib.request.Request(download_url, headers=headers)

        with urllib.request.urlopen(req, timeout=60) as response:
            status_code = response.getcode()
            content_length = int(response.headers.get("Content-Length", 0))

            if status_code == 206:  # HTTP 206 Partial Content
                total_bytes = existing_bytes + content_length
                mode = "ab"
                downloaded = existing_bytes
            else:  # HTTP 200 OK
                total_bytes = content_length
                mode = "wb"
                downloaded = 0

            with open(temp_path, mode) as out_file:
                block_size = 8 * 1024 * 1024  # 8MB chunks for maximum throughput
                while True:
                    chunk = response.read(block_size)
                    if not chunk:
                        break
                    out_file.write(chunk)
                    downloaded += len(chunk)
                    _update_progress(downloaded, total_bytes)

        progress_bar.empty()
        status_text.empty()

        if os.path.exists(temp_path) and os.path.getsize(temp_path) > 100_000_000:
            os.replace(temp_path, gaps_path)
            st.success("✅ Download complete! Initializing database engine...")
            st.rerun()
        else:
            downloaded_mb = os.path.getsize(temp_path) / (1024 * 1024) if os.path.exists(temp_path) else 0
            raise RuntimeError(f"Download incomplete ({downloaded_mb:.1f} MB fetched). Please refresh the page to resume.")

    except Exception as e:
        progress_bar.empty()
        status_text.empty()
        current_sz = os.path.getsize(temp_path) if os.path.exists(temp_path) else 0
        current_mb = current_sz / (1024 * 1024)
        st.error(
            f"❌ Database (`{gaps_path}`) download interrupted ({current_mb:.1f} MB downloaded so far).\n\n"
            f"**Error details:** {e}\n\n"
            f"💡 **To fix this:** Refresh the page to automatically resume the download from {current_mb:.1f} MB, or build locally with `cargo run --release --bin build_gaps -- {gap_k}`."
        )
        st.stop()

@st.cache_data(ttl=3600, show_spinner=False)
def resolve_direct_url(url: str) -> str:
    """Resolves HTTP 302 redirects to obtain direct S3/GitHub Object storage URL supporting HTTP Range requests."""
    if not (url.startswith("http://") or url.startswith("https://")):
        return url
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"}, method="HEAD")
        with urllib.request.urlopen(req, timeout=10) as resp:
            return resp.geturl()
    except Exception:
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
            with urllib.request.urlopen(req, timeout=10) as resp:
                return resp.geturl()
        except Exception:
            return url

# ============================================================================
# 3. Database Engine & Query Layer (DuckDB)
# ============================================================================

@st.cache_resource
def get_db_connection() -> duckdb.DuckDBPyConnection:
    conn = duckdb.connect()
    conn.sql("SET max_memory = '1GB';")
    conn.sql("SET threads = 2;")
    try:
        conn.sql("INSTALL httpfs; LOAD httpfs;")
    except Exception:
        pass
    return conn

@st.cache_data(show_spinner=False)
def fetch_dataset_metadata(_conn: duckdb.DuckDBPyConnection, gaps_target: str) -> DatasetMetadata:
    if not (gaps_target.startswith("http://") or gaps_target.startswith("https://")):
        if not os.path.exists(gaps_target) or os.path.getsize(gaps_target) <= 100_000:
            st.error(f"❌ Dataset file `{gaps_target}` is missing or corrupted.")
            st.stop()
    escaped_path = gaps_target.replace("'", "''")
    try:
        meta = _conn.sql(f"SELECT 1, COUNT(*), COUNT(*), COUNT(DISTINCT deltak) FROM read_parquet('{escaped_path}')").fetchone()
        return DatasetMetadata(
            min_idx=int(meta[0]),
            max_idx=int(meta[1]),
            total_count=int(meta[2]),
            unique_gaps_count=int(meta[3]),
        )
    except Exception as e:
        st.error(
            f"❌ Unable to connect or query dataset (`{gaps_target}`).\n\n"
            f"**Details:** {e}\n\n"
            f"💡 *If running on Streamlit Cloud, please refresh the page to retry or resume background caching.*"
        )
        st.stop()

@st.cache_data(show_spinner=False)
def query_prime_gaps(
    _conn: duckdb.DuckDBPyConnection,
    gaps_target: str,
    params: FilterParams,
) -> pd.DataFrame:
    """Queries k-step gap frequency distribution by prime index range [min_idx, max_idx]."""
    if not (gaps_target.startswith("http://") or gaps_target.startswith("https://")):
        if not os.path.exists(gaps_target) or os.path.getsize(gaps_target) <= 100_000:
            st.error(f"❌ Dataset file `{gaps_target}` is missing or corrupted.")
            st.stop()

    offset = params.min_idx - 1
    limit_count = params.max_idx - params.min_idx + 1
    escaped_path = gaps_target.replace("'", "''")

    query = f"""
    WITH sliced AS (
        SELECT deltak FROM read_parquet('{escaped_path}')
        LIMIT {limit_count} OFFSET {offset}
    )
    SELECT deltak AS diff, COUNT(*) AS frequency
    FROM sliced
    GROUP BY deltak
    ORDER BY frequency DESC
    LIMIT {params.top_n};
    """

    try:
        with _db_lock:
            return _conn.sql(query).df()
    except Exception as e:
        st.error(
            f"❌ Query execution failed on dataset (`{gaps_target}`).\n\n"
            f"**Details:** {e}\n\n"
            f"💡 *If running on Streamlit Cloud, please refresh the page to retry or resume background caching.*"
        )
        st.stop()

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

def render_telemetry_bar(dataset_target: str, range_count: int, elapsed_sec: float) -> None:
    """Renders real-time telemetry info for dataset engine, network usage, and latency."""
    is_remote = dataset_target.startswith("http://") or dataset_target.startswith("https://")

    if is_remote:
        engine_label = "🌐 Remote DuckDB HTTPS Stream (Zero-Copy)"
        est_transfer_mb = max(0.06, (range_count * 0.2) / (1024 * 1024))
        transfer_str = f"~{est_transfer_mb:.2f} MB"
    else:
        engine_label = "⚡ Local SSD File Path (Sub-10ms Speed)"
        transfer_str = "0.00 MB (100% Offline)"

    st.markdown("---")
    col1, col2, col3 = st.columns([2.2, 1.5, 1])
    with col1:
        st.caption(f"**Engine Mode:** {engine_label}")
    with col2:
        st.caption(f"**Est. Data Transfer:** `{transfer_str}`")
    with col3:
        st.caption(f"**Query Latency:** `{elapsed_sec * 1000:.1f} ms`")

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
            t0 = time.perf_counter()
            raw_df = query_prime_gaps(conn, config.gaps_file, params)
            elapsed_sec = time.perf_counter() - t0

            if raw_df.empty:
                st.warning("No prime pairs found in the selected index range.")
                return

            df = process_gap_dataframe(raw_df, params.sort_by)

            render_gap_distribution_chart(df, gap_k)
            render_data_table(df, gap_k)

            range_count = params.max_idx - params.min_idx + 1
            render_telemetry_bar(config.gaps_file, range_count, elapsed_sec)
    finally:
        st.session_state["is_processing"] = False
