# ============================================================================
# Main Application Entry Point Wrapper (Defaults to app2.py / k=2)
# ============================================================================
#
# Dedicated App Modules:
# - app2.py: 2-Step Prime Gap Explorer (k=2)
# - app3.py: 3-Step Prime Gap Explorer (k=3)
# - jumpchamp_web: Modular package containing config, database, UI components & runner
# - app_common.py: Backward-compatible re-export facade
#
import app2

if __name__ == "__main__":
    app2.main()