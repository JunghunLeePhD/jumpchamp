import pandas as pd
import plotly.express as px
import streamlit as st

from .config import DatasetMetadata, FilterParams

# ============================================================================
# View Components Layer (Streamlit UI)
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
        unsafe_allow_html=True,
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

    col1, col2_a, col2_b, col3, col4, col5 = st.columns([0.9, 0.8, 0.8, 1.0, 1.0, 1.8])

    with col1:
        sort_by = st.selectbox(
            "Sort Order",
            options=["Frequency", "Gap Size"],
            index=0,
            disabled=is_processing,
            help="'Frequency' sorts descending by count; 'Gap Size' orders numerically along the X-axis.",
        )

    with col2_a:
        top_min = st.number_input(
            "Min Rank",
            min_value=1,
            max_value=max(1, meta.unique_gaps_count),
            value=1,
            step=1,
            disabled=is_processing,
            help="Minimum rank of gaps to display (1 = most frequent)",
        )

    with col2_b:
        top_max = st.number_input(
            "Max Rank",
            min_value=1,
            max_value=max(1, meta.unique_gaps_count),
            value=min(20, meta.unique_gaps_count),
            step=1,
            disabled=is_processing,
            help="Maximum rank of gaps to display",
        )

    with col3:
        st.number_input(
            "Min Index (n)",
            min_value=meta.min_idx,
            max_value=meta.max_idx - 1,
            key="min_idx_val",
            on_change=sync_from_numbers,
            step=100_000,
            disabled=is_processing,
        )

    with col4:
        st.number_input(
            "Max Index (m)",
            min_value=meta.min_idx + 1,
            max_value=meta.max_idx,
            key="max_idx_val",
            on_change=sync_from_numbers,
            step=100_000,
            disabled=is_processing,
        )

    with col5:
        st.slider(
            "Prime Index Range (n to m)",
            min_value=meta.min_idx,
            max_value=meta.max_idx,
            key="slider_bounds",
            on_change=sync_from_slider,
            disabled=is_processing,
        )

    top_min_val = int(min(top_min, top_max))
    top_max_val = int(max(top_min, top_max))

    return FilterParams(
        min_idx=int(st.session_state.min_idx_val),
        max_idx=int(st.session_state.max_idx_val),
        top_min=top_min_val,
        top_max=top_max_val,
        sort_by=sort_by,
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
        color_continuous_scale="Viridis",
    )
    fig.update_traces(texttemplate='%{text}%', textposition='outside')
    fig.update_layout(
        xaxis_title=gap_label,
        yaxis_title="Frequency Count",
        showlegend=False,
        height=420,
        margin=dict(l=10, r=10, t=10, b=10),
        font=dict(family="Inter, system-ui, sans-serif", size=13),
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
