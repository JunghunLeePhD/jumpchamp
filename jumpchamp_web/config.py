from dataclasses import dataclass

# ============================================================================
# Domain Types & Configuration Layer
# ============================================================================

@dataclass(frozen=True)
class AppConfig:
    gaps_file: str
    release_url: str

@dataclass(frozen=True)
class FilterParams:
    min_idx: int
    max_idx: int
    top_min: int
    top_max: int
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
