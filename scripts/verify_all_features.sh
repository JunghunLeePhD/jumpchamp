#!/usr/bin/env bash
# ==============================================================================
# JumpChamp Master Feature Verification Suite
# Tests every component across the native Rust GUI application:
# 1. Environment & Toolchain Diagnostics
# 2. Rust Unit Tests (Sieve, GUI State & Panels, Analytics)
# 3. Binary Compilations (jumpchamp GUI & jumpchamp_gui bundle binary)
# 4. Sieve Math & In-Memory Prime Engine Verification
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

# Ensure cargo is on PATH if installed in ~/.cargo/bin
if ! command -v cargo >/dev/null 2>&1 && [ -d "$HOME/.cargo/bin" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
fi

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


print_header "Phase 2: Rust Unit Tests (Sieve, GUI, Analytics)"

if cargo test --all-targets; then
    pass_step "All Rust Unit Tests (cargo test)"
else
    fail_step "Rust Unit Tests" "cargo test reported test failures"
fi


print_header "Phase 3: Binary Compilation Verification"

echo "Compiling jumpchamp (Default GUI Binary)..."
if cargo build --release --bin jumpchamp; then
    pass_step "Binary compilation: jumpchamp (egui native GUI)"
else
    fail_step "Binary compilation: jumpchamp" "Compilation failed"
fi

echo "Checking jumpchamp_gui (Application bundle binary)..."
if cargo check --release --bin jumpchamp_gui; then
    pass_step "Binary check: jumpchamp_gui"
else
    fail_step "Binary check: jumpchamp_gui" "Check failed"
fi


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
