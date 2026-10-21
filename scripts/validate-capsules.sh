#!/usr/bin/env bash
set -euo pipefail

echo "🔍 Validating SHA-256 integrity of 225 Master Capsules..."
MANIFEST=".alexandria/capsules/alexandria-capsules.manifest.json"

if [ ! -f "$MANIFEST" ]; then
    echo "❌ Manifest file not found at $MANIFEST"
    exit 1
fi

COUNT=$(find .alexandria/capsules/ -name "*.capsule.json" | wc -l)
echo "✅ Found $COUNT capsules in .alexandria/capsules/"

cargo test --test skill_collector_test
echo "✅ All capsule ETL and manifest verification tests passed!"
