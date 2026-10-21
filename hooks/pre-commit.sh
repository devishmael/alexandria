#!/usr/bin/env bash
set -euo pipefail

echo "🛡️ Alexandria Pre-Commit Quality Gate Scan..."

# Gate 1: Check compilation
if command -v cargo >/dev/null 2>&1; then
    cargo check --all-targets --quiet
fi

# Run test suite
if command -v cargo >/dev/null 2>&1; then
    cargo test --all-targets --quiet
fi

echo "✅ All Alexandria Quality Gates cleared."
exit 0
