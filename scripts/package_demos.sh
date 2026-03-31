#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname "$0")/.." && pwd)
CRATES_DIR="$ROOT_DIR/crates"
OUTPUT_DIR="$ROOT_DIR/demos"

if ! command -v zip >/dev/null 2>&1; then
    echo "zip not found; skipping demo packaging."
    exit 0
fi

mkdir -p "$CRATES_DIR" "$OUTPUT_DIR"
find "$OUTPUT_DIR" -mindepth 1 -maxdepth 1 -name '*.gtbundle' -delete
find "$OUTPUT_DIR" -mindepth 1 -maxdepth 1 -name '*.gtpack' -delete

shopt -s nullglob
crate_manifests=("$CRATES_DIR"/*/Cargo.toml)
packaged_any=0

if [ ${#crate_manifests[@]} -eq 0 ]; then
    echo "No demo crates found under crates/. Nothing to package."
    exit 0
fi

for manifest in "${crate_manifests[@]}"; do
    crate_dir=$(dirname "$manifest")
    demo_name=$(basename "$crate_dir")
    bundle_path="$OUTPUT_DIR/$demo_name.gtbundle"
    pack_dirs=("$crate_dir"/bundle/packs/*.pack)

    if [ ! -f "$crate_dir/bundle/bundle.yaml" ]; then
        echo "Skipping $demo_name: no bundle workspace found at $crate_dir/bundle" >&2
        continue
    fi

    if (
        cd "$crate_dir/bundle"
        zip -rq "$bundle_path" .
    ); then
        artifact_kind=$(file -b "$bundle_path" || true)
        if ! printf '%s' "$artifact_kind" | grep -qi 'zip archive'; then
            echo "Skipping $demo_name: expected ZIP .gtbundle but got: ${artifact_kind:-unknown}" >&2
            rm -f "$bundle_path"
            continue
        fi
        echo "Created demos/$demo_name.gtbundle"
        packaged_any=1
    else
        echo "Skipping $demo_name: bundle packaging failed" >&2
    fi

    if [ ${#pack_dirs[@]} -eq 0 ]; then
        continue
    fi

    for pack_dir in "${pack_dirs[@]}"; do
        pack_name="$(basename "$pack_dir" .pack)"
        pack_path="$OUTPUT_DIR/$pack_name.gtpack"

        if (
            cd "$pack_dir"
            zip -rq "$pack_path" .
        ); then
            artifact_kind=$(file -b "$pack_path" || true)
            if ! printf '%s' "$artifact_kind" | grep -qi 'zip archive'; then
                echo "Skipping $pack_name: expected ZIP .gtpack but got: ${artifact_kind:-unknown}" >&2
                rm -f "$pack_path"
                continue
            fi
            echo "Created demos/$pack_name.gtpack"
            packaged_any=1
        else
            echo "Skipping $pack_name: pack packaging failed" >&2
        fi
    done
done

if [ "$packaged_any" -eq 0 ]; then
    echo "No demo artifacts were packaged successfully." >&2
    exit 1
fi
