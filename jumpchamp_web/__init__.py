# ============================================================================
# JumpChamp Web Package Root
# ============================================================================

from .config import AppConfig, FilterParams, DatasetMetadata, DEFAULT_RELEASE_URL_TEMPLATE, load_config
from .ingestion import ensure_dataset_exists, resolve_direct_url
from .database import get_db_connection, fetch_dataset_metadata, query_prime_gaps, process_gap_dataframe
from .components import (
    inject_sticky_navbar_css,
    render_math_definitions,
    render_top_filter_bar,
    render_gap_distribution_chart,
    render_data_table,
    render_telemetry_bar,
)
from .runner import run_app

__all__ = [
    "AppConfig",
    "FilterParams",
    "DatasetMetadata",
    "DEFAULT_RELEASE_URL_TEMPLATE",
    "load_config",
    "ensure_dataset_exists",
    "resolve_direct_url",
    "get_db_connection",
    "fetch_dataset_metadata",
    "query_prime_gaps",
    "process_gap_dataframe",
    "inject_sticky_navbar_css",
    "render_math_definitions",
    "render_top_filter_bar",
    "render_gap_distribution_chart",
    "render_data_table",
    "render_telemetry_bar",
    "run_app",
]
