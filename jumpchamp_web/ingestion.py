import os
import urllib.request
import streamlit as st

from .config import AppConfig

# ============================================================================
# Asset Ingestion Layer
# ============================================================================

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
