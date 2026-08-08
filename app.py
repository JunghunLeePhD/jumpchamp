# ============================================================================
# Main Application Entry Point Wrapper (Defaults to app2.py / k=2)
# ============================================================================
#
# Dedicated App Modules:
# - app2.py: 2-Step Prime Gap Explorer (k=2)
# - app3.py: 3-Step Prime Gap Explorer (k=3)
# - app_common.py: Shared UI renderers, DuckDB queries & orchestration
#
import app2

if __name__ == "__main__":
    app2.main()