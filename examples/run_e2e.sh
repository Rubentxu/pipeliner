#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUSTLINE="cargo run --quiet -p pipeliner-cli --bin pipeliner-cli --"

# Or if using the root binary:
# RUSTLINE="$SCRIPT_DIR/../target/debug/rustline"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass=0
fail=0
skip=0

assert_exit() {
    local desc="$1"
    local expected="$2"
    shift 2
    set +e
    output=$("$@" 2>&1)
    actual=$?
    set -e
    if [ "$actual" -eq "$expected" ]; then
        echo -e "${GREEN}PASS${NC}: $desc"
        ((pass++))
    else
        echo -e "${RED}FAIL${NC}: $desc (expected exit=$expected, got exit=$actual)"
        echo "  Output: $output"
        ((fail++))
    fi
}

assert_output() {
    local desc="$1"
    local expected_pattern="$2"
    shift 2
    set +e
    output=$("$@" 2>&1)
    exit_code=$?
    set -e
    if echo "$output" | grep -q "$expected_pattern"; then
        echo -e "${GREEN}PASS${NC}: $desc"
        ((pass++))
    else
        echo -e "${RED}FAIL${NC}: $desc (pattern '$expected_pattern' not found)"
        echo "  Output: $output"
        ((fail++))
    fi
}

echo "========================================"
echo "Pipeliner E2E Test Suite"
echo "========================================"
echo ""

# ========================================
# Section 1: CLI Help & Version
# ========================================
echo "--- CLI Basics ---"

assert_exit "help shows successfully" 0 $RUSTLINE --help
assert_output "help mentions script command" "script" $RUSTLINE --help
assert_output "help mentions run command" "run" $RUSTLINE --help
assert_exit "version shows successfully" 0 $RUSTLINE --version

# ========================================
# Section 2: Script Execution (Rust DSL)
# ========================================
echo ""
echo "--- Rust Script Execution ---"

# Test 01: Hello world script
assert_output "hello script prints greeting" "Hello from Pipeliner" $RUSTLINE script "$SCRIPT_DIR/scripts/01-hello.rs"

# Test 02: Env vars script
assert_output "env-vars script shows pipeline context" "Pipeline:" $RUSTLINE script "$SCRIPT_DIR/scripts/02-env-vars.rs"

# Test 03: Script with deps
assert_output "deps script outputs JSON" '"pipeline"' $RUSTLINE script "$SCRIPT_DIR/scripts/03-with-deps.rs"

# Test 04: Build and test script
assert_output "build-test script runs stages" "Build Stage" $RUSTLINE script "$SCRIPT_DIR/scripts/04-build-and-test.rs"

# Test 05: Error handling script (should fail with exit 1)
assert_exit "error-handling script exits with code 1" 1 $RUSTLINE script "$SCRIPT_DIR/scripts/05-error-handling.rs"

# ========================================
# Section 3: Pipeline JSON Execution
# ========================================
echo ""
echo "--- Pipeline JSON Execution ---"

# Test simple pipeline
assert_output "simple pipeline runs" "Building project" $RUSTLINE run --file "$SCRIPT_DIR/pipelines/01-simple.json"

# Test multi-stage pipeline
assert_output "multi-stage pipeline runs" "Compiling" $RUSTLINE run --file "$SCRIPT_DIR/pipelines/02-with-scripts.json"

# ========================================
# Section 4: CLI Commands
# ========================================
echo ""
echo "--- CLI Commands ---"

# Validate
assert_exit "validate accepts valid pipeline" 0 $RUSTLINE validate --file "$SCRIPT_DIR/pipelines/01-simple.json"

# Check
assert_exit "check accepts valid pipeline" 0 $RUSTLINE check --file "$SCRIPT_DIR/pipelines/01-simple.json"

# Init
TMPDIR=$(mktemp -d)
assert_exit "init creates pipeline file" 0 $RUSTLINE init --name "test-pipeline" --output "$TMPDIR/pipeline.json"
assert_output "init file contains pipeline name" "test-pipeline" cat "$TMPDIR/pipeline.json"
rm -rf "$TMPDIR"

# Script with nonexistent file
assert_exit "nonexistent script fails" 1 $RUSTLINE script "$SCRIPT_DIR/scripts/nonexistent.rs"

# Script with non-.rs extension
echo "not a rust script" > /tmp/test.txt
assert_exit "non-rs file fails" 1 $RUSTLINE script /tmp/test.txt
rm -f /tmp/test.txt

# ========================================
# Summary
# ========================================
echo ""
echo "========================================"
echo "Results: $pass passed, $fail failed, $skip skipped"
echo "========================================"

if [ "$fail" -gt 0 ]; then
    exit 1
fi