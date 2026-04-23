#!/usr/bin/env bash

set -euo pipefail

usage() {
    cat <<'EOF'
Usage: scripts/publish_demo.sh <demo-name> [--tag <release-tag>] [--repo <owner/name>] [--dry-run]

Publishes a single demo's release assets to GitHub:
  - demos/<demo-name>-create-answers.json
  - demos/<demo-name>-setup-answers.json (if present)
  - demos/<pack-file>.gtpack

The uploaded create-answers asset is normalized to point at:
  https://github.com/<repo>/releases/latest/download/<pack-file>.gtpack
EOF
}

ROOT_DIR=$(cd -- "$(dirname "$0")/.." && pwd)
DEMOS_DIR="$ROOT_DIR/demos"
CRATES_DIR="$ROOT_DIR/crates"
TMP_DIR=""

cleanup() {
    if [[ -n "$TMP_DIR" && -d "$TMP_DIR" ]]; then
        rm -rf "$TMP_DIR"
    fi
}
trap cleanup EXIT

require_cmd() {
    local cmd="$1"
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "$cmd not found" >&2
        exit 1
    fi
}

resolve_repo() {
    gh repo view --json nameWithOwner --jq '.nameWithOwner'
}

resolve_latest_release_tag() {
    gh api "repos/$(resolve_repo)/releases" \
    | jq -r 'map(select(.draft | not)) | map(select(.prerelease | not)) | .[0].tag_name // empty'
}

extract_pack_filename() {
    local create_answers="$1"
    local reference

    reference="$(jq -r '.answers.delegate_answer_document.answers.app_pack_entries[0].reference // empty' "$create_answers")"
    if [[ -z "$reference" ]]; then
        reference="$(jq -r '.answers.delegate_answer_document.answers.app_packs[0] // empty' "$create_answers")"
    fi

    reference="${reference#file://}"
    reference="${reference##*/}"

    if [[ -z "$reference" || "$reference" == "null" ]]; then
        return 1
    fi

    printf '%s\n' "$reference"
}

build_pack_if_needed() {
    local demo_name="$1"
    local pack_path="$2"
    local crate_dir="$CRATES_DIR/$demo_name"
    local build_script="$crate_dir/build_pack.sh"
    local build_answers="$crate_dir/build-answer.json"

    if [[ -f "$pack_path" ]]; then
        return 0
    fi

    if [[ -x "$build_script" ]]; then
        "$build_script" "$pack_path"
        return 0
    fi

    if [[ -f "$build_answers" ]]; then
        "$ROOT_DIR/scripts/package_demos.sh" "$demo_name" >/dev/null
        if [[ -f "$pack_path" ]]; then
            return 0
        fi
    fi

    echo "Missing pack artifact: $pack_path" >&2
    echo "No shared build-answer.json or custom build script produced $pack_path" >&2
    exit 1
}

normalize_create_answers() {
    local source_path="$1"
    local target_path="$2"
    local release_pack_url="$3"

    jq \
      --arg release_pack_url "$release_pack_url" \
      '
      .answers.delegate_answer_document.answers.app_pack_entries |= map(
        .reference = $release_pack_url
        | .detected_kind = "https"
      )
      | .answers.delegate_answer_document.answers.app_packs |= map($release_pack_url)
      ' \
      "$source_path" > "$target_path"
}

upload_and_verify() {
    local repo="$1"
    local tag="$2"
    shift 2
    local files=("$@")

    gh release upload "$tag" "${files[@]}" --clobber --repo "$repo"

    local release_assets
    release_assets="$(gh release view "$tag" --repo "$repo" --json assets --jq '.assets[].name')"

    local missing=0
    local file
    for file in "${files[@]}"; do
        local asset_name
        asset_name="$(basename "$file")"
        if ! grep -Fxq "$asset_name" <<<"$release_assets"; then
            echo "Missing uploaded release asset: $asset_name" >&2
            missing=1
        fi
    done

    if [[ "$missing" -ne 0 ]]; then
        exit 1
    fi
}

DEMO_NAME=""
RELEASE_TAG=""
REPO=""
DRY_RUN=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --tag)
            RELEASE_TAG="${2:-}"
            shift 2
            ;;
        --repo)
            REPO="${2:-}"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        -*)
            echo "Unknown option: $1" >&2
            usage >&2
            exit 1
            ;;
        *)
            if [[ -n "$DEMO_NAME" ]]; then
                echo "Only one demo name may be provided" >&2
                usage >&2
                exit 1
            fi
            DEMO_NAME="$1"
            shift
            ;;
    esac
done

if [[ -z "$DEMO_NAME" ]]; then
    usage >&2
    exit 1
fi

require_cmd gh
require_cmd jq

if [[ -z "$REPO" ]]; then
    REPO="$(resolve_repo)"
fi

if [[ -z "$RELEASE_TAG" ]]; then
    RELEASE_TAG="$(resolve_latest_release_tag)"
fi

if [[ -z "$RELEASE_TAG" ]]; then
    echo "Could not resolve a GitHub release tag. Pass --tag <release-tag>." >&2
    exit 1
fi

CREATE_ANSWERS_PATH="$DEMOS_DIR/${DEMO_NAME}-create-answers.json"
SETUP_ANSWERS_PATH="$DEMOS_DIR/${DEMO_NAME}-setup-answers.json"

if [[ ! -f "$CREATE_ANSWERS_PATH" ]]; then
    echo "Missing create answers: $CREATE_ANSWERS_PATH" >&2
    exit 1
fi

PACK_FILENAME="$(extract_pack_filename "$CREATE_ANSWERS_PATH")"
PACK_PATH="$DEMOS_DIR/$PACK_FILENAME"

build_pack_if_needed "$DEMO_NAME" "$PACK_PATH"

if [[ ! -f "$PACK_PATH" ]]; then
    echo "Pack artifact was not produced: $PACK_PATH" >&2
    exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/publish-demo.XXXXXX")"
NORMALIZED_CREATE_ANSWERS="$TMP_DIR/$(basename "$CREATE_ANSWERS_PATH")"
RELEASE_PACK_URL="https://github.com/${REPO}/releases/latest/download/${PACK_FILENAME}"

normalize_create_answers "$CREATE_ANSWERS_PATH" "$NORMALIZED_CREATE_ANSWERS" "$RELEASE_PACK_URL"

FILES_TO_UPLOAD=("$NORMALIZED_CREATE_ANSWERS")
if [[ -f "$SETUP_ANSWERS_PATH" ]]; then
    FILES_TO_UPLOAD+=("$SETUP_ANSWERS_PATH")
fi
FILES_TO_UPLOAD+=("$PACK_PATH")

echo "Demo: $DEMO_NAME"
echo "Repo: $REPO"
echo "Release tag: $RELEASE_TAG"
echo "Pack: $PACK_FILENAME"
printf 'Upload:\n'
printf '  %s\n' "${FILES_TO_UPLOAD[@]}"

if [[ "$DRY_RUN" -eq 1 ]]; then
    exit 0
fi

upload_and_verify "$REPO" "$RELEASE_TAG" "${FILES_TO_UPLOAD[@]}"

echo "Published ${DEMO_NAME} assets to release ${RELEASE_TAG}"
