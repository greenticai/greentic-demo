#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname "$0")" && pwd)
REPO_ROOT=$(cd -- "$ROOT_DIR/../.." && pwd)
OUT_PATH="${1:-$REPO_ROOT/demos/deep-research-demo.gtpack}"
TMP_DIR=$(mktemp -d "${TMPDIR:-/tmp}/deep-research-pack.XXXXXX")
PACK_DIR="$TMP_DIR/deep-research-demo.pack"
ANSWERS_PATH="$ROOT_DIR/.pack_answers.build.json"
LOCAL_GREENTIC_PACK="$REPO_ROOT/../greentic-pack/target/debug/greentic-pack"
LOCAL_GREENTIC_FLOW="$REPO_ROOT/../greentic-flow/target/debug/greentic-flow"

if [[ -x "$LOCAL_GREENTIC_PACK" ]]; then
    GREENTIC_PACK_BIN="$LOCAL_GREENTIC_PACK"
else
    GREENTIC_PACK_BIN="greentic-pack"
fi

if [[ -x "$LOCAL_GREENTIC_FLOW" ]]; then
    FLOW_BIN_DIR="$(dirname "$LOCAL_GREENTIC_FLOW")"
else
    FLOW_BIN_DIR=""
fi

cleanup() {
    rm -f "$ANSWERS_PATH"
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

jq --arg pack_dir "$PACK_DIR" '
  .answers.pack_dir = $pack_dir
' "$ROOT_DIR/pack_answers.json" > "$ANSWERS_PATH"

(
    cd "$ROOT_DIR"
    PATH="$FLOW_BIN_DIR:$(dirname "$GREENTIC_PACK_BIN"):$PATH" "$GREENTIC_PACK_BIN" wizard apply --answers "$ANSWERS_PATH" >/dev/null
    cd "$PACK_DIR"
    PATH="$FLOW_BIN_DIR:$(dirname "$GREENTIC_PACK_BIN"):$PATH" "$GREENTIC_PACK_BIN" resolve --in . >/dev/null
    PATH="$FLOW_BIN_DIR:$(dirname "$GREENTIC_PACK_BIN"):$PATH" "$GREENTIC_PACK_BIN" build --in . >/dev/null
)

mkdir -p "$(dirname "$OUT_PATH")"
cp "$PACK_DIR/dist/deep-research-demo.pack.gtpack" "$OUT_PATH"
echo "created $OUT_PATH"
