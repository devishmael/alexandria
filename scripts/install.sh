#!/usr/bin/env bash
set -euo pipefail

echo "🏛️ Installing Alexandria Kernel..."

if command -v cargo >/dev/null 2>&1; then
    echo "⚙️ Found Cargo. Installing from source repository..."
    cargo install --git https://github.com/devishmael/alexandria.git
    echo "✅ Alexandria installed successfully to $(which alexandria || echo '~/.cargo/bin/alexandria')"
    alexandria -v
else
    echo "❌ Cargo is required to build Alexandria from source."
    echo "Please install Rust via https://rustup.rs and rerun this script."
    exit 1
fi
