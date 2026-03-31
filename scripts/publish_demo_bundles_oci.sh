#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname "$0")/.." && pwd)
OUTPUT_DIR="$ROOT_DIR/demos"
ARTIFACTS_DIR="${ARTIFACTS_DIR:-$ROOT_DIR/.artifacts}"
OWNER="${OWNER:?OWNER is required}"
SHA="${SHA:?SHA is required}"
REF_NAME="${REF_NAME:-}"
REF_TYPE="${REF_TYPE:-}"
BRANCH_NAME="${BRANCH_NAME:-}"
PUBLISH_VERSION="${PUBLISH_VERSION:-}"

if ! command -v oras >/dev/null 2>&1; then
    echo "oras not found; skipping OCI bundle publication."
    exit 0
fi

mkdir -p "$ARTIFACTS_DIR"
shopt -s nullglob
bundles=("$OUTPUT_DIR"/*.gtbundle)

if [ ${#bundles[@]} -eq 0 ]; then
    echo "No .gtbundle artifacts found under demos/. Nothing to publish."
    exit 0
fi

for bundle_path in "${bundles[@]}"; do
    bundle_name="$(basename "$bundle_path" .gtbundle)"
    media_type="application/vnd.greentic.${bundle_name}.bundle.v1+tar+gzip"

    sha_ref="ghcr.io/${OWNER}/bundles/${bundle_name}:${SHA}"
    echo "Publishing ${bundle_name} bundle..."
    oras push --disable-path-validation "$sha_ref" "${bundle_path}:${media_type}"
    echo "${bundle_name}=${sha_ref}" >> "$ARTIFACTS_DIR/bundle-refs.txt"
    echo "  -> ${sha_ref}"

    if [[ "$BRANCH_NAME" == "main" || "$BRANCH_NAME" == "master" ]]; then
        latest_ref="ghcr.io/${OWNER}/bundles/${bundle_name}:latest"
        oras push --disable-path-validation "$latest_ref" "${bundle_path}:${media_type}"
        echo "${bundle_name}_latest=${latest_ref}" >> "$ARTIFACTS_DIR/bundle-refs.txt"
        echo "  -> ${latest_ref}"
    fi

    if [[ -n "$PUBLISH_VERSION" ]]; then
        version_ref="ghcr.io/${OWNER}/bundles/${bundle_name}:${PUBLISH_VERSION}"
        oras push --disable-path-validation "$version_ref" "${bundle_path}:${media_type}"
        echo "${bundle_name}_version=${version_ref}" >> "$ARTIFACTS_DIR/bundle-refs.txt"
        echo "  -> ${version_ref}"
    fi
done
