#!/usr/bin/env bash
# migrate_demo_assets.sh
# Compares a demo crate's bundle/packs/*/assets/ against its top-level assets/
# and either reports (dry-run) or auto-merges (--execute) files that exist
# only in the nested location. Refuses to overwrite (exit 1) on conflict.
#
# Optional --check-components mode flags WASM files under components/ that
# are not referenced from any source/config file in the crate.
#
# Usage:
#   migrate_demo_assets.sh <demo-name> [--root <dir>] [--execute]
#   migrate_demo_assets.sh <demo-name> [--root <dir>] --check-components
#
# --root defaults to the script's parent crates/ dir.
#
# Exit codes:
#   0 success / clean
#   1 conflict (different content same path) — manual resolve required
#   2 invocation error

set -euo pipefail

usage() {
    cat <<EOF
Usage: $0 <demo-name> [--root <dir>] [--execute|--check-components]
EOF
    exit 2
}

DEMO=""
ROOT=""
MODE="dry-run"

while [ $# -gt 0 ]; do
    case "$1" in
        --root) ROOT="$2"; shift 2 ;;
        --execute) MODE="execute"; shift ;;
        --check-components) MODE="check-components"; shift ;;
        -h|--help) usage ;;
        *)
            if [ -z "$DEMO" ]; then DEMO="$1"; shift
            else echo "unexpected arg: $1" >&2; usage
            fi
            ;;
    esac
done

[ -n "$DEMO" ] || usage

if [ -z "$ROOT" ]; then
    SCRIPT_DIR=$(cd -- "$(dirname "$0")" && pwd)
    ROOT="$SCRIPT_DIR/../crates"
fi

CRATE="$ROOT/$DEMO"
[ -d "$CRATE" ] || { echo "demo not found: $CRATE" >&2; exit 2; }

if [ "$MODE" = "check-components" ]; then
    COMPONENTS="$CRATE/components"
    [ -d "$COMPONENTS" ] || { echo "no components/ dir, nothing to check"; exit 0; }
    found=0
    while IFS= read -r -d '' wasm; do
        name=$(basename "$wasm")
        if grep -RFq --exclude-dir=bundle --exclude-dir=components --exclude-dir=target --exclude-dir=.git "$name" "$CRATE" 2>/dev/null; then
            echo "REFERENCED $name"
        else
            echo "CACHE $name"
            found=$((found + 1))
        fi
    done < <(find "$COMPONENTS" -type f -name '*.wasm' -print0)
    echo "Summary: $found unreferenced WASM file(s) flagged as CACHE"
    exit 0
fi

NESTED_ASSETS_GLOB="$CRATE/bundle/packs/*/assets"
TOP_ASSETS="$CRATE/assets"

mkdir -p "$TOP_ASSETS"

merges=0
conflicts=0

shopt -s nullglob
for nested_dir in $NESTED_ASSETS_GLOB; do
    [ -d "$nested_dir" ] || continue
    while IFS= read -r -d '' nested_file; do
        rel="${nested_file#"$nested_dir/"}"
        top_file="$TOP_ASSETS/$rel"
        if [ -f "$top_file" ]; then
            if ! cmp -s "$nested_file" "$top_file"; then
                echo "CONFLICT $rel"
                conflicts=$((conflicts + 1))
            fi
        else
            echo "MERGE $rel"
            merges=$((merges + 1))
            if [ "$MODE" = "execute" ]; then
                mkdir -p "$(dirname "$top_file")"
                cp -n "$nested_file" "$top_file"
            fi
        fi
    done < <(find "$nested_dir" -type f -print0)
done
shopt -u nullglob

echo "Summary: $merges merge(s), $conflicts conflict(s) (mode=$MODE)"

[ "$conflicts" -eq 0 ] || exit 1
exit 0
