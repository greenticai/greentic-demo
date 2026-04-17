#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname "$0")" && pwd)
REPO_ROOT=$(cd -- "$ROOT_DIR/../.." && pwd)
OUT_PATH="${1:-$REPO_ROOT/demos/deep-research-demo.gtpack}"
TMP_DIR=$(mktemp -d "${TMPDIR:-/tmp}/deep-research-pack.XXXXXX")
PACK_DIR="$TMP_DIR/deep-research-demo.pack"
LOCAL_GREENTIC_PACK="$REPO_ROOT/../greentic-pack/target/debug/greentic-pack"

if [[ -x "$LOCAL_GREENTIC_PACK" ]]; then
    GREENTIC_PACK_BIN="$LOCAL_GREENTIC_PACK"
else
    GREENTIC_PACK_BIN="greentic-pack"
fi

cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

cp "$ROOT_DIR/pack_answers.json" "$TMP_DIR/pack_answers.json"
cp -R "$ROOT_DIR/assets" "$TMP_DIR/"

(
    cd "$TMP_DIR"
    PATH="$(dirname "$GREENTIC_PACK_BIN"):$PATH" "$GREENTIC_PACK_BIN" wizard apply --answers "$TMP_DIR/pack_answers.json" >/dev/null
    cd "$PACK_DIR"
    PATH="$(dirname "$GREENTIC_PACK_BIN"):$PATH" "$GREENTIC_PACK_BIN" build --in . >/dev/null
)

mkdir -p "$(dirname "$OUT_PATH")"
cp "$PACK_DIR/dist/deep-research-demo.pack.gtpack" "$OUT_PATH"
echo "created $OUT_PATH"
