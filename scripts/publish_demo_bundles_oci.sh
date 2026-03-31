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
    echo "oras not found; skipping OCI demo artifact publication."
    exit 0
fi

mkdir -p "$ARTIFACTS_DIR"
: > "$ARTIFACTS_DIR/bundle-refs.txt"
: > "$ARTIFACTS_DIR/pack-refs.txt"
shopt -s nullglob
bundles=("$OUTPUT_DIR"/*.gtbundle)
packs=("$OUTPUT_DIR"/*.gtpack)

if [ ${#bundles[@]} -eq 0 ] && [ ${#packs[@]} -eq 0 ]; then
    echo "No demo artifacts found under demos/. Nothing to publish."
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

for pack_path in "${packs[@]}"; do
    pack_name="$(basename "$pack_path" .gtpack)"
    media_type="application/vnd.greentic.gtpack.v1+zip"

    sha_ref="ghcr.io/${OWNER}/packs/demos/${pack_name}:${SHA}"
    echo "Publishing ${pack_name} pack..."
    oras push --disable-path-validation "$sha_ref" "${pack_path}:${media_type}"
    echo "${pack_name}=${sha_ref}" >> "$ARTIFACTS_DIR/pack-refs.txt"
    echo "  -> ${sha_ref}"

    if [[ "$BRANCH_NAME" == "main" || "$BRANCH_NAME" == "master" ]]; then
        latest_ref="ghcr.io/${OWNER}/packs/demos/${pack_name}:latest"
        oras push --disable-path-validation "$latest_ref" "${pack_path}:${media_type}"
        echo "${pack_name}_latest=${latest_ref}" >> "$ARTIFACTS_DIR/pack-refs.txt"
        echo "  -> ${latest_ref}"
    fi

    if [[ -n "$PUBLISH_VERSION" ]]; then
        version_ref="ghcr.io/${OWNER}/packs/demos/${pack_name}:${PUBLISH_VERSION}"
        oras push --disable-path-validation "$version_ref" "${pack_path}:${media_type}"
        echo "${pack_name}_version=${version_ref}" >> "$ARTIFACTS_DIR/pack-refs.txt"
        echo "  -> ${version_ref}"
    fi
done
