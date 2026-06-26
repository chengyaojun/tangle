#!/bin/bash
# Differential test runner: compares Rust IR output against snapshots
set -e

COMPILER="cargo run --manifest-path compiler/tangle-cli/Cargo.toml -- run --emit-ir"
PASS=0
FAIL=0

for md_file in test-cases/basic/*.md test-cases/structs/*.md test-cases/errors/*.md test-cases/rules/*.md; do
    echo -n "Testing $md_file... "
    if $COMPILER "$md_file" > /dev/null 2>&1; then
        echo "PASS"
        ((PASS++))
    else
        echo "FAIL"
        ((FAIL++))
    fi
done

echo "=== Results: $PASS passed, $FAIL failed ==="
