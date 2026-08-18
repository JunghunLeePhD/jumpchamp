import os
import threading
import duckdb
import pandas as pd
import streamlit as st

from .config import DatasetMetadata, FilterParams

_db_lock = threading.Lock()

# ============================================================================
# Database Engine & Query Layer (DuckDB)
# ============================================================================

@st.cache_resource
def get_db_connection() -> duckdb.DuckDBPyConnection:
    conn = duckdb.connect()
    num_threads = min(4, max(2, os.cpu_count() or 2))
    conn.sql("SET max_memory = '1GB';")
    conn.sql(f"SET threads = {num_threads};")
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
        meta = _conn.sql(
            f"SELECT 1, COUNT(*), COUNT(*), COUNT(DISTINCT deltak) FROM read_parquet('{escaped_path}')"
        ).fetchone()
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

    top_limit = max(1, params.top_max - params.top_min + 1)
    top_offset = max(0, params.top_min - 1)

    query = f"""
    WITH sliced AS (
        SELECT deltak FROM read_parquet('{escaped_path}')
        LIMIT {limit_count} OFFSET {offset}
    )
    SELECT deltak AS diff, COUNT(*) AS frequency
    FROM sliced
    GROUP BY deltak
    ORDER BY frequency DESC
    LIMIT {top_limit} OFFSET {top_offset};
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
