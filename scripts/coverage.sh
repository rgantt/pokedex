#!/bin/bash
# Generate code coverage report for all tests (unit + screenplay)
# Requires: cargo-llvm-cov, llvm-tools-preview
#
# Usage:
#   ./scripts/coverage.sh          # summary only
#   ./scripts/coverage.sh --html   # open HTML report in browser
#   ./scripts/coverage.sh --lcov   # generate coverage.lcov file

set -euo pipefail

POKEDEX_BIN=$(which pokedex 2>/dev/null || echo "")

echo "=== Cleaning previous coverage data ==="
cargo llvm-cov clean --workspace

echo "=== Building instrumented binary ==="
cargo llvm-cov test --no-report --test validate_encounters --no-run 2>&1 | tail -3

INSTRUMENTED="target/llvm-cov-target/debug/pokedex"
if [ ! -f "$INSTRUMENTED" ]; then
    echo "Error: instrumented binary not found at $INSTRUMENTED"
    exit 1
fi

echo "=== Installing instrumented binary ==="
mkdir -p ~/.local/bin
cp "$INSTRUMENTED" ~/.local/bin/pokedex
export PATH="$HOME/.local/bin:$PATH"

# Seed DB if needed
if [ ! -f ~/.pokedex/db.sqlite ]; then
    echo "=== Seeding database ==="
    pokedex db seed 2>&1 | tail -3
fi

echo "=== Running validate_encounters tests ==="
cargo llvm-cov test --no-report --test validate_encounters 2>&1 | tail -3

echo "=== Running screenplay tests ==="
cargo llvm-cov test --no-report --test run_screenplays 2>&1 | tail -3

echo ""
echo "=== Coverage Summary ==="
cargo llvm-cov report --summary-only

if [ "${1:-}" = "--html" ]; then
    echo "=== Generating HTML report ==="
    cargo llvm-cov report --html
    echo "Report at: target/llvm-cov/html/index.html"
    open target/llvm-cov/html/index.html 2>/dev/null || xdg-open target/llvm-cov/html/index.html 2>/dev/null || true
elif [ "${1:-}" = "--lcov" ]; then
    cargo llvm-cov report --lcov --output-path coverage.lcov
    echo "LCOV report written to coverage.lcov"
fi

# Restore original binary if it existed
if [ -n "$POKEDEX_BIN" ] && [ "$POKEDEX_BIN" != "$HOME/.local/bin/pokedex" ]; then
    echo "Note: ~/.local/bin/pokedex is now the instrumented binary. Reinstall with ./install.sh for production use."
fi
