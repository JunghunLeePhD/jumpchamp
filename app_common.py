# ============================================================================
# JumpChamp Web Common — Backward-Compatible Re-export Facade
# ============================================================================
#
# All modularized components live under the `jumpchamp_web/` package:
# - jumpchamp_web.config      — Domain configuration types & loaders
# - jumpchamp_web.ingestion   — Resumable Range downloads & URL resolution
# - jumpchamp_web.database    — DuckDB query engine & data processing
# - jumpchamp_web.components  — Streamlit UI components & Plotly charts
# - jumpchamp_web.runner      — Main application orchestrator
#

from jumpchamp_web import (
    AppConfig,
    DatasetMetadata,
    DEFAULT_RELEASE_URL_TEMPLATE,
    FilterParams,
    ensure_dataset_exists,
    fetch_dataset_metadata,
    get_db_connection,
    inject_sticky_navbar_css,
    load_config,
    process_gap_dataframe,
    query_prime_gaps,
    render_data_table,
    render_gap_distribution_chart,
    render_math_definitions,
    render_telemetry_bar,
    render_top_filter_bar,
    resolve_direct_url,
    run_app,
)

__all__ = [
    "AppConfig",
    "DatasetMetadata",
    "DEFAULT_RELEASE_URL_TEMPLATE",
    "FilterParams",
    "ensure_dataset_exists",
    "fetch_dataset_metadata",
    "get_db_connection",
    "inject_sticky_navbar_css",
    "load_config",
    "process_gap_dataframe",
    "query_prime_gaps",
    "render_data_table",
    "render_gap_distribution_chart",
    "render_math_definitions",
    "render_telemetry_bar",
    "render_top_filter_bar",
    "resolve_direct_url",
    "run_app",
]
