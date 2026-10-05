#!/usr/bin/env bash
# ==============================================================================
# JumpChamp Master Feature Verification Suite
# Tests every component and feature across Rust and Python subsystems:
# 1. Environment & Toolchain Diagnostics
# 2. Rust Unit Tests (Sieve, Storage, Analytics)
# 3. Binary Compilations (jumpchamp, build_primes, build_gaps, jumpchamp_gui)
# 4. Pipeline E2E (primes.parquet, gaps2.parquet, gaps3.parquet generation)
# 5. CLI Gap Distribution Analyzer (Fast Path & Slow Path)
# 6. Python Web & DuckDB Engine Tests
# 7. Streamlit Web Dashboard Headless Health Check
# ==============================================================================

set -eo pipefail

BOLD="\033[1m"
GREEN="\033[0;32m"
RED="\033[0;31m"
YELLOW="\033[0;33m"
CYAN="\033[0;36m"
NC="\033[0m"

PASSED_COUNT=0
FAILED_COUNT=0
TOTAL_TESTS=0

pass_step() {
    local name="$1"
    echo -e "${GREEN}✔ [PASS]${NC} ${name}"
    PASSED_COUNT=$((PASSED_COUNT + 1))
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
}

fail_step() {
    local name="$1"
    local detail="$2"
    echo -e "${RED}✖ [FAIL]${NC} ${name}: ${detail}"
    FAILED_COUNT=$((FAILED_COUNT + 1))
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
}

print_header() {
    echo ""
    echo -e "${CYAN}==============================================================================${NC}"
    echo -e "${BOLD}${CYAN}  $1${NC}"
    echo -e "${CYAN}==============================================================================${NC}"
}

WORKSPACE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${WORKSPACE_DIR}"

print_header "Phase 1: Environment & Toolchain Diagnostics"

ARCH=$(uname -m)
OS_DESC=$(cat /etc/os-release 2>/dev/null | grep PRETTY_NAME | cut -d= -f2 | tr -d '"' || uname -s)
echo -e "• Machine Architecture : ${BOLD}${ARCH}${NC}"
echo -e "• Operating System     : ${BOLD}${OS_DESC}${NC}"

if command -v rustc >/dev/null 2>&1 && command -v cargo >/dev/null 2>&1; then
    RUST_VER=$(rustc --version)
    pass_step "Rust Toolchain: ${RUST_VER}"
else
    fail_step "Rust Toolchain" "cargo / rustc not found in PATH"
fi

if command -v python3 >/dev/null 2>&1; then
    PY_VER=$(python3 --version)
    pass_step "Python Environment: ${PY_VER}"
else
    fail_step "Python Environment" "python3 not found in PATH"
fi

if python3 -c "import streamlit, duckdb, plotly, pandas" >/dev/null 2>&1; then
    pass_step "Python Libraries: streamlit, duckdb, plotly, pandas present"
else
    fail_step "Python Libraries" "Failed to import required Python dependencies"
fi


print_header "Phase 2: Rust Unit Tests (Sieve, Storage, Analytics)"

if cargo test --all-targets; then
    pass_step "All Rust Unit Tests (cargo test)"
else
    fail_step "Rust Unit Tests" "cargo test reported test failures"
fi


print_header "Phase 3: Binary Compilation Verification"

echo "Compiling build_primes..."
if cargo build --release --bin build_primes; then
    pass_step "Binary compilation: build_primes"
else
    fail_step "Binary compilation: build_primes" "Compilation failed"
fi

echo "Compiling build_gaps..."
if cargo build --release --bin build_gaps; then
    pass_step "Binary compilation: build_gaps"
else
    fail_step "Binary compilation: build_gaps" "Compilation failed"
fi

echo "Compiling jumpchamp (CLI Analyzer)..."
if cargo build --release --bin jumpchamp; then
    pass_step "Binary compilation: jumpchamp CLI"
else
    fail_step "Binary compilation: jumpchamp CLI" "Compilation failed"
fi

echo "Checking jumpchamp_gui (egui / eframe desktop)..."
if cargo check --release --bin jumpchamp_gui; then
    pass_step "Binary check: jumpchamp_gui (egui desktop UI)"
else
    fail_step "Binary check: jumpchamp_gui" "Check failed"
fi


print_header "Phase 4: End-to-End Pipeline & Parquet Generation"

TEST_LIMIT=1000000
echo -e "Generating test primes up to N=${TEST_LIMIT}..."
rm -f primes.parquet gaps2.parquet gaps3.parquet test_gaps_fixture.parquet

if cargo run --release --bin build_primes -- "${TEST_LIMIT}"; then
    if [ -f "primes.parquet" ] && [ -s "primes.parquet" ]; then
        SIZE=$(du -h primes.parquet | cut -f1)
        pass_step "build_primes: created primes.parquet (${SIZE})"
    else
        fail_step "build_primes" "primes.parquet was not created or empty"
    fi
else
    fail_step "build_primes" "Execution failed"
fi

echo "Precomputing k=2 gaps (gaps2.parquet)..."
if cargo run --release --bin build_gaps -- 2; then
    if [ -f "gaps2.parquet" ] && [ -s "gaps2.parquet" ]; then
        SIZE=$(du -h gaps2.parquet | cut -f1)
        pass_step "build_gaps (k=2): created gaps2.parquet (${SIZE})"
    else
        fail_step "build_gaps (k=2)" "gaps2.parquet missing"
    fi
else
    fail_step "build_gaps (k=2)" "Execution failed"
fi

echo "Precomputing k=3 gaps (gaps3.parquet)..."
if cargo run --release --bin build_gaps -- 3; then
    if [ -f "gaps3.parquet" ] && [ -s "gaps3.parquet" ]; then
        SIZE=$(du -h gaps3.parquet | cut -f1)
        pass_step "build_gaps (k=3): created gaps3.parquet (${SIZE})"
    else
        fail_step "build_gaps (k=3)" "gaps3.parquet missing"
    fi
else
    fail_step "build_gaps (k=3)" "Execution failed"
fi


print_header "Phase 5: CLI Gap Distribution Analyzer Execution"

echo "Executing Fast Path using gaps2.parquet (k=2, range 1 to 50,000)..."
if cargo run --release -- 2 1 50000; then
    pass_step "CLI Analyzer Fast Path (gaps2.parquet zero-copy stream)"
else
    fail_step "CLI Analyzer Fast Path" "Execution failed"
fi

echo "Executing Slow Path using primes.parquet --force (k=2, range 1 to 50,000)..."
if cargo run --release -- 2 1 50000 primes.parquet --force; then
    pass_step "CLI Analyzer Slow Path (primes.parquet --force sliding window)"
else
    fail_step "CLI Analyzer Slow Path" "Execution failed"
fi


print_header "Phase 6: Python Web Pipeline & DuckDB Integration Tests"

if python3 -m unittest discover -s tests -p "test_*.py"; then
    pass_step "Python Unit & Integration Test Suite"
else
    fail_step "Python Unit & Integration Test Suite" "Test assertions failed"
fi


print_header "Phase 7: Streamlit Web Dashboard Headless Smoke Test"

STREAMLIT_PID=""
cleanup_streamlit() {
    if [ -n "${STREAMLIT_PID}" ] && kill -0 "${STREAMLIT_PID}" 2>/dev/null; then
        echo "Stopping headless Streamlit server (PID ${STREAMLIT_PID})..."
        kill "${STREAMLIT_PID}" 2>/dev/null || true
        wait "${STREAMLIT_PID}" 2>/dev/null || true
    fi
}
trap cleanup_streamlit EXIT

echo "Starting Streamlit in headless background mode on port 8501..."
streamlit run app2.py --server.headless true --server.port 8501 --server.enableCORS false --server.enableXsrfProtection false >/tmp/streamlit_smoke.log 2>&1 &
STREAMLIT_PID=$!

HEALTH_OK=false
for i in {1..20}; do
    if curl -sf http://localhost:8501/_stcore/health >/dev/null 2>&1; then
        HEALTH_OK=true
        break
    fi
    sleep 0.5
done

if [ "${HEALTH_OK}" = true ]; then
    pass_step "Streamlit Web Dashboard (HTTP 200 on /_stcore/health)"
else
    fail_step "Streamlit Web Dashboard" "Health check did not respond within 10s. Log: $(tail -n 10 /tmp/streamlit_smoke.log)"
fi

cleanup_streamlit
STREAMLIT_PID=""


print_header "Feature Verification Summary"

echo -e "Total Checks : ${BOLD}${TOTAL_TESTS}${NC}"
echo -e "Passed       : ${BOLD}${GREEN}${PASSED_COUNT}${NC}"
echo -e "Failed       : ${BOLD}${RED}${FAILED_COUNT}${NC}"
echo ""

if [ "${FAILED_COUNT}" -eq 0 ]; then
    echo -e "${BOLD}${GREEN}🎉 ALL FEATURES VERIFIED SUCCESSFULLY IN THIS ENVIRONMENT!${NC}"
    exit 0
else
    echo -e "${BOLD}${RED}⚠️ SOME CHECKS FAILED. Please review the log output above.${NC}"
    exit 1
fi
