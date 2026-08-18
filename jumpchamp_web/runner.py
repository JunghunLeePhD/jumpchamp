import time
import streamlit as st

from .config import load_config
from .ingestion import ensure_dataset_exists
from .database import get_db_connection, fetch_dataset_metadata, query_prime_gaps, process_gap_dataframe
from .components import (
    inject_sticky_navbar_css,
    render_math_definitions,
    render_top_filter_bar,
    render_gap_distribution_chart,
    render_data_table,
    render_telemetry_bar,
)

# ============================================================================
# Main Application Orchestrator
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
