#!/usr/bin/env bash
# Smoke tests for migrate_demo_assets.sh
# Builds an in-memory fixture demo dir under /tmp and asserts script behavior.

set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname "$0")/.." && pwd)
SCRIPT="$ROOT_DIR/scripts/migrate_demo_assets.sh"
FIXTURE_ROOT="${TMPDIR:-/tmp}/migrate-test-demo"

PASS=0
FAIL=0

build_fixture_clean() {
    rm -rf "$FIXTURE_ROOT"
    mkdir -p "$FIXTURE_ROOT/crates/sample-demo/assets/cards"
    mkdir -p "$FIXTURE_ROOT/crates/sample-demo/bundle/packs/sample.pack/assets/cards"
    echo '{"id":"a"}' > "$FIXTURE_ROOT/crates/sample-demo/assets/cards/a.json"
    echo '{"id":"a"}' > "$FIXTURE_ROOT/crates/sample-demo/bundle/packs/sample.pack/assets/cards/a.json"
}

build_fixture_with_extra_in_nested() {
    build_fixture_clean
    echo '{"id":"b"}' > "$FIXTURE_ROOT/crates/sample-demo/bundle/packs/sample.pack/assets/cards/b.json"
}

build_fixture_with_conflict() {
    build_fixture_clean
    echo '{"id":"a","mutated":true}' > "$FIXTURE_ROOT/crates/sample-demo/bundle/packs/sample.pack/assets/cards/a.json"
}

assert_eq() {
    local label="$1" expected="$2" actual="$3"
    if [ "$expected" = "$actual" ]; then
        echo "PASS: $label"
        PASS=$((PASS + 1))
    else
        echo "FAIL: $label (expected=$expected actual=$actual)"
        FAIL=$((FAIL + 1))
    fi
}

# Test 1: clean state, dry-run = exit 0, no copies reported
build_fixture_clean
if out=$("$SCRIPT" sample-demo --root "$FIXTURE_ROOT/crates" 2>&1); then rc=0; else rc=$?; fi
assert_eq "clean dry-run exit code" "0" "$rc"
assert_eq "clean dry-run reports no merges" "0" "$(echo "$out" | grep -c '^MERGE ')"

# Test 2: dry-run with extra-in-nested = exit 0, reports MERGE for b.json
build_fixture_with_extra_in_nested
if out=$("$SCRIPT" sample-demo --root "$FIXTURE_ROOT/crates" 2>&1); then rc=0; else rc=$?; fi
assert_eq "extra dry-run exit code" "0" "$rc"
assert_eq "extra dry-run merges 1 file" "1" "$(echo "$out" | grep -c '^MERGE ')"
[ ! -f "$FIXTURE_ROOT/crates/sample-demo/assets/cards/b.json" ] && \
    { echo "PASS: dry-run did not actually copy"; PASS=$((PASS + 1)); } || \
    { echo "FAIL: dry-run copied file"; FAIL=$((FAIL + 1)); }

# Test 3: --execute with extra-in-nested = exit 0, file copied
build_fixture_with_extra_in_nested
if out=$("$SCRIPT" sample-demo --root "$FIXTURE_ROOT/crates" --execute 2>&1); then rc=0; else rc=$?; fi
assert_eq "execute exit code" "0" "$rc"
[ -f "$FIXTURE_ROOT/crates/sample-demo/assets/cards/b.json" ] && \
    { echo "PASS: --execute copied file"; PASS=$((PASS + 1)); } || \
    { echo "FAIL: --execute did not copy file"; FAIL=$((FAIL + 1)); }

# Test 4: --execute is idempotent
if out2=$("$SCRIPT" sample-demo --root "$FIXTURE_ROOT/crates" --execute 2>&1); then rc2=0; else rc2=$?; fi
assert_eq "execute idempotent exit code" "0" "$rc2"

# Test 5: conflict = exit 1
build_fixture_with_conflict
if out=$("$SCRIPT" sample-demo --root "$FIXTURE_ROOT/crates" 2>&1); then rc=0; else rc=$?; fi
assert_eq "conflict exit code" "1" "$rc"
echo "$out" | grep -q '^CONFLICT ' && \
    { echo "PASS: conflict reported"; PASS=$((PASS + 1)); } || \
    { echo "FAIL: conflict not reported"; FAIL=$((FAIL + 1)); }

# Test 6: --check-components on demo with referenced WASM = no flag
build_fixture_clean
mkdir -p "$FIXTURE_ROOT/crates/sample-demo/components"
printf '\0asm\1\0\0\0' > "$FIXTURE_ROOT/crates/sample-demo/components/used.wasm"
echo '{"answers":{"component":"components/used.wasm"}}' > "$FIXTURE_ROOT/crates/sample-demo/gtc_wizard_answers.json"
if out=$("$SCRIPT" sample-demo --root "$FIXTURE_ROOT/crates" --check-components 2>&1); then rc=0; else rc=$?; fi
assert_eq "check-components referenced exit code" "0" "$rc"
assert_eq "check-components referenced no flag" "0" "$(echo "$out" | grep -c '^CACHE ')"

# Test 7: --check-components on unreferenced WASM = flagged
printf '\0asm\1\0\0\0' > "$FIXTURE_ROOT/crates/sample-demo/components/orphan.wasm"
if out=$("$SCRIPT" sample-demo --root "$FIXTURE_ROOT/crates" --check-components 2>&1); then rc=0; else rc=$?; fi
assert_eq "check-components orphan exit code" "0" "$rc"
assert_eq "check-components flags orphan" "1" "$(echo "$out" | grep -c '^CACHE orphan.wasm')"

echo
echo "Total: $((PASS + FAIL))  Passed: $PASS  Failed: $FAIL"
[ "$FAIL" -eq 0 ]
