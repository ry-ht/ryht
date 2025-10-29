#!/usr/bin/env bash
#
# Test script for reproducing Qdrant startup issues
# Usage: ./test_db_cycles.sh

set -e

export PATH="/Users/taaliman/.cargo/bin:/usr/local/bin:/usr/bin:/bin"

echo "=== Cleaning environment ==="
pkill -9 -f qdrant 2>/dev/null || true
pkill -9 -f surreal 2>/dev/null || true
sleep 2

echo "=== Clearing logs ==="
rm -f ~/.cortex/logs/*.log

echo ""
echo "=== Test 1: Fresh start ==="
./dist/cortex db start || { echo "FAILED: Test 1"; exit 1; }

echo ""
echo "=== Test 2: Start while running (should handle gracefully) ==="
./dist/cortex db start || { echo "FAILED: Test 2"; exit 1; }

echo ""
echo "=== Test 3: Stop ==="
./dist/cortex db stop || { echo "FAILED: Stop"; exit 1; }
sleep 2

echo ""
echo "=== Test 4: Start after stop ==="
./dist/cortex db start || { echo "FAILED: Test 4"; exit 1; }

echo ""
echo "=== Test 5: Stop again ==="
./dist/cortex db stop || { echo "FAILED: Stop 2"; exit 1; }
sleep 2

echo ""
echo "=== Test 6: Start after second stop ==="
./dist/cortex db start || { echo "FAILED: Test 6"; exit 1; }

echo ""
echo "=== Cleanup ==="
./dist/cortex db stop

echo ""
echo "=== ALL TESTS PASSED ==="
echo ""
echo "If you saw failures, please send:"
echo "1. The exact test number that failed"
echo "2. Output of: cat ~/.cortex/logs/qdrant.stderr.log"
echo "3. Output of: lsof -i :6333"
echo "4. Output of: pgrep -f qdrant"
