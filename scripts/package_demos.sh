#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname "$0")/.." && pwd)
CRATES_DIR="$ROOT_DIR/crates"
OUTPUT_DIR="$ROOT_DIR/demos"

if ! command -v greentic-bundle >/dev/null 2>&1; then
    echo "greentic-bundle is required to package real .gtbundle artifacts" >&2
    exit 1
fi

mkdir -p "$CRATES_DIR" "$OUTPUT_DIR"
find "$OUTPUT_DIR" -mindepth 1 -maxdepth 1 -name '*.gtbundle' -delete

shopt -s nullglob
crate_manifests=("$CRATES_DIR"/*/Cargo.toml)

if [ ${#crate_manifests[@]} -eq 0 ]; then
    echo "No demo crates found under crates/. Nothing to package."
    exit 0
fi

for manifest in "${crate_manifests[@]}"; do
    crate_dir=$(dirname "$manifest")
    demo_name=$(basename "$crate_dir")
    bundle_path="$OUTPUT_DIR/$demo_name.gtbundle"

    if [ ! -f "$crate_dir/bundle/bundle.yaml" ]; then
        echo "Skipping $demo_name: no bundle workspace found at $crate_dir/bundle" >&2
        continue
    fi

    greentic-bundle build --root "$crate_dir/bundle" --offline --output "$bundle_path" >/dev/null
    echo "Created demos/$demo_name.gtbundle"
done
