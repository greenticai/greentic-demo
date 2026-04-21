#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname "$0")" && pwd)
COMPONENT_DIR="$ROOT_DIR/component-telco-present"
STAGE_DIR="$ROOT_DIR/generated-pack/components/component-telco-present"

mkdir -p "$STAGE_DIR"

cargo component build \
  --release \
  --target wasm32-wasip2 \
  --manifest-path "$COMPONENT_DIR/Cargo.toml" >/dev/null

cp "$COMPONENT_DIR/target/wasm32-wasip2/release/component_telco_present.wasm" "$STAGE_DIR/component.wasm"
cp "$COMPONENT_DIR/component.manifest.json" "$STAGE_DIR/component.manifest.json"
