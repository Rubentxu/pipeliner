#!/usr/bin/env bash
# Pipeline CI/CD con Pipeliner
# Compilar + Test + Verificar binary + Stats

set -euo pipefail

PIPELINE_NAME="${1:-pipeliner}"
BINARY="${2:-target/debug/$PIPELINE_NAME}"

echo "=== Pipeline: $PIPELINE_NAME"
echo "Binary: $BINARY"

# Build
echo "Build..."
cargo build --release --bin "$PIPELINE_NAME"

# Tests
echo "Tests..."
cargo test --all || exit 1

# Verify binary
if [ ! -f "$BINARY" ]; then
    echo "Binary no encontrado: $BINARY"
    exit 1
fi

# Stats
echo ""
echo "=== Stats"
ls -lh "$BINARY"
stat "$BINARY"
echo "Build: SUCCESS"
