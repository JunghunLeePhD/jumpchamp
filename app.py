import os
import urllib.request
import duckdb
import pandas as pd
import plotly.express as px
import streamlit as st
from dotenv import load_dotenv

# Load local .env file if present
load_dotenv()

def get_config(key, default_value=""):
    # 1. Check OS environment variables (loaded via .env)
    env_val = os.getenv(key)
    if env_val:
        return env_val
    
    # 2. Safely check Streamlit Cloud secrets
    try:
        if key in st.secrets:
            return st.secrets[key]
    except Exception:
        # Fails silently locally when no secrets.toml file exists
        pass
        
    # 3. Fallback default
    return default_value
# ============================================================================
# 1. Configuration & Environment Variables
# ============================================================================

st.set_page_config(
    page_title="Prime Gap Explorer",
    page_icon="🦀",
    layout="wide"
)

# Read configuration safely
PARQUET_FILE = get_config("PARQUET_FILE_PATH", "primes.parquet")
RELEASE_URL = get_config(
    "RELEASE_URL", 
    "https://github.com/JunghunLeePhD/primes/releases/download/v1.0.0/primes.parquet"
)

# ============================================================================
# 2. Auto-Download Helper for Large Files (500 MB+)
# ============================================================================

def ensure_database_exists():
    """Downloads primes.parquet from GitHub Releases if not present locally."""
    if os.path.exists(PARQUET_FILE):
        return

    st.info(f"📦 Database file (`{PARQUET_FILE}`) not found locally.")
    st.write("Downloading database from remote storage... *(This only happens once on initial startup)*")

    progress_bar = st.progress(0.0)
    status_text = st.empty()

    def progress_callback(block_num, block_size, total_size):
        downloaded = block_num * block_size
        if total_size > 0:
            percent = min(1.0, downloaded / total_size)
            progress_bar.progress(percent)
            downloaded_mb = downloaded / (1024 * 1024)
            total_mb = total_size / (1024 * 1024)
            status_text.text(f"Downloading: {downloaded_mb:.1f} MB / {total_mb:.1f} MB ({int(percent * 100)}%)")
        else:
            status_text.text(f"Downloaded {downloaded / (1024 * 1024):.1f} MB...")

    try:
        urllib.request.urlretrieve(RELEASE_URL, PARQUET_FILE, reporthook=progress_callback)
        progress_bar.empty()
        status_text.empty()
        st.success("✅ Download complete! Initializing database...")
        st.rerun()
    except Exception as e:
        progress_bar.empty()
        status_text.empty()
        st.error(f"❌ Failed to download dataset from URL:\n`{RELEASE_URL}`\n\nError details: {e}")
        st.warning("Please check your `RELEASE_URL` in `.env` or ensure `primes.parquet` is present in the app root.")
        st.stop()

# Ensure parquet file is present before continuing
ensure_database_exists()

# ============================================================================
# 3. DuckDB Database Connection & Memory Safety
# ============================================================================

st.title("🦀 Prime Gap Distribution Explorer")

@st.cache_resource
def get_duckdb_connection():
    conn = duckdb.connect()
    # 🔒 Prevent DuckDB from consuming all system RAM and triggering OOM Killer
    conn.sql("SET max_memory = '1GB';")
    conn.sql("SET threads = 2;")
    return conn

conn = get_duckdb_connection()

# Query dataset bounds in O(1) time
metadata = conn.sql(f"""
    SELECT 
        MIN(prime) as min_p, 
        MAX(prime) as max_p, 
        COUNT(*) as total_count 
    FROM '{PARQUET_FILE}'
""").fetchone()

db_min_p, db_max_p, db_total_count = int(metadata[0]), int(metadata[1]), int(metadata[2])

# ============================================================================
# 4. Sidebar Controls & Inputs
# ============================================================================

st.sidebar.header("⚙️ Filter Parameters")

# Database Overview
st.sidebar.metric("Total Primes in DB", f"{db_total_count:,}")
st.sidebar.metric("Prime Range", f"{db_min_p:,} to {db_max_p:,}")
st.sidebar.markdown("---")

# User Controls
k = st.sidebar.number_input(
    "Step Size (k)",
    min_value=1,
    max_value=20,
    value=2,
    help="Computes the difference between p_{n+k} and p_n."
)

# Default to 1,000,000 or max available to prevent RAM overload on startup
default_initial_max = min(db_max_p, 1_000_000)

st.sidebar.subheader("Select Prime Interval Bounds")
min_prime = st.sidebar.number_input(
    "Min Prime (A)",
    min_value=db_min_p,
    max_value=db_max_p - 1,
    value=db_min_p
)

max_prime = st.sidebar.number_input(
    "Max Prime (B)",
    min_value=min_prime + 1,
    max_value=db_max_p,
    value=default_initial_max
)

# ⚠️ Make sure top_n is defined HERE before Section 5
top_n = st.sidebar.slider("Top N Gaps to Show", min_value=5, max_value=50, value=20)

# ============================================================================
# 5. Fast DuckDB Query Execution
# ============================================================================

with st.spinner("Executing C++ DuckDB engine query..."):
    # Convert inputs to clean integers
    k_val = int(k)
    min_p_val = int(min_prime)
    max_p_val = int(max_prime)
    top_n_val = int(top_n)

    query = f"""
    WITH interval_primes AS (
        SELECT prime 
        FROM '{PARQUET_FILE}'
        WHERE prime BETWEEN {min_p_val} AND {max_p_val}
    ),
    gaps AS (
        SELECT 
            LEAD(prime, {k_val}) OVER (ORDER BY prime) - prime AS diff
        FROM interval_primes
    )
    SELECT 
        diff, 
        COUNT(*) AS frequency
    FROM gaps
    WHERE diff IS NOT NULL
    GROUP BY diff
    ORDER BY frequency DESC
    LIMIT {top_n_val};
    """
    
    df = conn.sql(query).df()
# ============================================================================
# 6. Data Formatting & Visualizations
# ============================================================================

total_pairs = df['frequency'].sum()
df['percentage'] = (df['frequency'] / total_pairs * 100).round(2)
df['diff_label'] = df['diff'].astype(str)

# KPI Summary Cards
col1, col2, col3 = st.columns(3)
most_frequent_gap = int(df.iloc[0]['diff'])
highest_pct = df.iloc[0]['percentage']

col1.metric("Selected Step Size", f"k = {k}")
col2.metric("Most Frequent Gap", f"Δ = {most_frequent_gap}")
col3.metric("Top Gap Percentage", f"{highest_pct}%")

st.markdown("---")

# Layout: Interactive Chart + Data Table
left_col, right_col = st.columns([2, 1])

with left_col:
    st.subheader(f"Frequency Distribution for $p_{{n+{k}}} - p_n$")
    
    fig = px.bar(
        df,
        x="diff_label",
        y="frequency",
        text="percentage",
        labels={"diff_label": f"Difference ($p_{{n+{k}}} - p_n$)", "frequency": "Frequency"},
        hover_data={"diff_label": True, "frequency": ":,", "percentage": ":.2f%"},
        color="frequency",
        color_continuous_scale="Viridis"
    )
    
    fig.update_traces(texttemplate='%{text}%', textposition='outside')
    fig.update_layout(
        xaxis_title=f"Gap Size ($p_{{n+{k}}} - p_n$)",
        yaxis_title="Count",
        xaxis={'type': 'category'},
        showlegend=False,
        height=500
    )
    
    # Updated: replaced use_container_width=True with width="stretch"
    st.plotly_chart(fig, width="stretch")

with right_col:
    st.subheader("📊 Data Table")
    
    display_df = df[['diff', 'frequency', 'percentage']].copy()
    display_df.columns = ['Gap (Diff)', 'Frequency', 'Percentage']
    display_df['Frequency'] = display_df['Frequency'].map('{:,}'.format)
    display_df['Percentage'] = display_df['Percentage'].map('{:.2f}%'.format)
    
    # Updated: replaced use_container_width=True with width="stretch"
    st.dataframe(display_df, hide_index=True, width="stretch")

    # CSV Export Button
    csv_data = display_df.to_csv(index=False).encode('utf-8')
    st.download_button(
        label="📥 Download Table as CSV",
        data=csv_data,
        file_name=f"prime_gaps_k{k}_interval_{min_prime}_{max_prime}.csv",
        mime="text/csv"
    )