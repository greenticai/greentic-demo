#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname "$0")/.." && pwd)
CRATES_DIR="$ROOT_DIR/crates"
DEMOS_DIR="$ROOT_DIR/demos"
TMP_ROOT="${TMPDIR:-/tmp}/greentic-demo-package"
DEFAULT_PACK_ANSWERS="$TMP_ROOT/pack-update-answers.json"

if ! command -v greentic-pack >/dev/null 2>&1; then
    echo "greentic-pack not found; skipping demo packaging."
    exit 0
fi

if ! command -v gtc >/dev/null 2>&1; then
    echo "gtc not found; skipping demo packaging."
    exit 0
fi

if ! command -v jq >/dev/null 2>&1; then
    echo "jq not found; skipping demo packaging."
    exit 0
fi

mkdir -p "$CRATES_DIR" "$DEMOS_DIR"
rm -rf "$TMP_ROOT"
mkdir -p "$TMP_ROOT"
find "$DEMOS_DIR" -mindepth 1 -maxdepth 1 -name '*.gtbundle' -delete
find "$DEMOS_DIR" -mindepth 1 -maxdepth 1 -name '*.gtpack' -delete

cat > "$DEFAULT_PACK_ANSWERS" <<'EOF'
{
  "wizard_id": "greentic-pack.wizard.run",
  "schema_id": "greentic-pack.wizard.answers",
  "schema_version": "1.0.0",
  "locale": "en-GB",
  "answers": {
    "dry_run": false,
    "mode": "interactive",
    "pack_dir": ".",
    "run_build": true,
    "run_delegate_component": false,
    "run_delegate_flow": false,
    "run_doctor": false,
    "selected_actions": [
      "main.update_application_pack",
      "update_application_pack.run_update_validate",
      "pipeline.update_validate",
      "pipeline.sign_prompt.skip",
      "main.exit"
    ],
    "sign": false
  },
  "locks": {}
}
EOF

shopt -s nullglob
pack_dirs=("$CRATES_DIR"/*/bundle/packs/*.pack)
generated_pack_answers=("$CRATES_DIR"/*/gtc_pack_create_wizard_answers.json)
bundle_answers=("$DEMOS_DIR"/*-create-answers.json)
packaged_any=0

if [ ${#pack_dirs[@]} -eq 0 ] && [ ${#generated_pack_answers[@]} -eq 0 ]; then
    echo "No demo pack sources or generated-pack answers found under crates/. Nothing to package."
    exit 1
fi

for source_pack_dir in "${pack_dirs[@]}"; do
    pack_name="$(basename "$source_pack_dir" .pack)"
    crate_dir="$(cd "$source_pack_dir/../../.." && pwd)"
    pack_answers="$DEFAULT_PACK_ANSWERS"
    create_answers="$crate_dir/gtc_pack_create_wizard_answers.json"
    temp_pack_dir="$TMP_ROOT/packs/$pack_name"
    built_pack="$temp_pack_dir/dist/$pack_name.gtpack"
    target_pack="$DEMOS_DIR/$pack_name.gtpack"

    if [ -f "$crate_dir/gtc_pack_wizard_answers.json" ]; then
        pack_answers="$crate_dir/gtc_pack_wizard_answers.json"
    fi

    if [ -f "$create_answers" ]; then
        temp_pack_parent="$TMP_ROOT/packs-create/$pack_name"
        temp_pack_dir="$temp_pack_parent/$pack_name.pack"
        built_pack="$temp_pack_dir/dist/$pack_name.gtpack"

        rm -rf "$temp_pack_parent"
        mkdir -p "$temp_pack_parent"

        if ! (
            cd "$temp_pack_parent"
            greentic-pack wizard apply --answers "$create_answers" >/dev/null
        ); then
            echo "Skipping $pack_name: pack scaffold wizard create failed" >&2
            continue
        fi

        if [ -d "$source_pack_dir/assets" ]; then
            mkdir -p "$temp_pack_dir/assets"
            cp -R "$source_pack_dir/assets/." "$temp_pack_dir/assets/"
        fi

        if [ -d "$source_pack_dir/components" ]; then
            mkdir -p "$temp_pack_dir/components"
            cp -R "$source_pack_dir/components/." "$temp_pack_dir/components/"
        fi

        if ! (
            cd "$temp_pack_dir"
            greentic-pack wizard apply --answers "$pack_answers" >/dev/null
        ); then
            echo "Skipping $pack_name: pack wizard build failed after scaffold replay" >&2
            continue
        fi
    else
        rm -rf "$temp_pack_dir"
        mkdir -p "$(dirname "$temp_pack_dir")"
        cp -R "$source_pack_dir" "$temp_pack_dir"

        if ! (
            cd "$temp_pack_dir"
            greentic-pack wizard apply --answers "$pack_answers" >/dev/null
        ); then
            echo "Skipping $pack_name: pack wizard build failed" >&2
            continue
        fi
    fi

    if [ ! -f "$built_pack" ]; then
        echo "Skipping $pack_name: wizard did not produce $built_pack" >&2
        continue
    fi

    cp "$built_pack" "$target_pack"
    echo "Created demos/$pack_name.gtpack"
    packaged_any=1
done

for create_answers in "${generated_pack_answers[@]}"; do
    crate_dir="$(cd "$(dirname "$create_answers")" && pwd)"
    pack_answers="$crate_dir/gtc_pack_wizard_answers.json"
    source_assets_dir="$crate_dir/assets"
    source_components_dir="$crate_dir/components"
    pack_dir_name="$(jq -r '.answers.pack_dir' "$create_answers" | xargs basename)"
    pack_id="$(jq -r '.answers.create_pack_id' "$create_answers")"
    pack_name="${pack_id%.pack}"

    if compgen -G "$crate_dir/bundle/packs/*.pack" >/dev/null; then
        continue
    fi

    if [ ! -f "$pack_answers" ]; then
        echo "Skipping $pack_name: missing pack wizard answers" >&2
        continue
    fi

    temp_pack_parent="$TMP_ROOT/packs-create/$pack_name"
    temp_pack_dir="$temp_pack_parent/$pack_dir_name"
    built_pack="$temp_pack_dir/dist/$pack_dir_name.gtpack"
    target_pack="$DEMOS_DIR/$pack_name.gtpack"

    rm -rf "$temp_pack_parent"
    mkdir -p "$temp_pack_parent"

    if ! (
        cd "$temp_pack_parent"
        greentic-pack wizard apply --answers "$create_answers" >/dev/null
    ); then
        echo "Skipping $pack_name: pack scaffold wizard create failed" >&2
        continue
    fi

    if [ -d "$source_assets_dir" ]; then
        mkdir -p "$temp_pack_dir/assets"
        cp -R "$source_assets_dir/." "$temp_pack_dir/assets/"
    fi

    if [ -d "$source_components_dir" ]; then
        mkdir -p "$temp_pack_dir/components"
        cp -R "$source_components_dir/." "$temp_pack_dir/components/"
    fi

    if ! (
        cd "$temp_pack_dir"
        greentic-pack wizard apply --answers "$pack_answers" >/dev/null
    ); then
        echo "Skipping $pack_name: pack wizard build failed after scaffold replay" >&2
        continue
    fi

    if [ ! -f "$built_pack" ]; then
        echo "Skipping $pack_name: wizard did not produce $built_pack" >&2
        continue
    fi

    cp "$built_pack" "$target_pack"
    echo "Created demos/$pack_name.gtpack"
    packaged_any=1
done

if [ ${#bundle_answers[@]} -eq 0 ]; then
    echo "No demo bundle answer files found under demos/." >&2
    exit 1
fi

for source_answers in "${bundle_answers[@]}"; do
    demo_basename="$(basename "$source_answers" -create-answers.json)"
    temp_answers="$TMP_ROOT/${demo_basename}-bundle-answers.json"
    output_dir="$TMP_ROOT/${demo_basename}-bundle"
    bundle_id="$(jq -r '.answers.delegate_answer_document.answers.bundle_id' "$source_answers")"
    built_bundle="$output_dir/dist/${bundle_id}.gtbundle"
    target_bundle="$DEMOS_DIR/${bundle_id}.gtbundle"

    jq --arg demos_dir "$DEMOS_DIR" --arg out "$output_dir" '
      .answers.delegate_answer_document.answers.output_dir = $out
      | .answers.delegate_answer_document.answers.app_pack_entries |= map(
          if (.reference | test("/download/[^/]+\\.gtpack$")) then
            .reference as $ref
            | ($ref | capture("/download/(?<file>[^/]+\\.gtpack)$").file) as $file
            | .reference = ($demos_dir + "/" + $file)
            | .detected_kind = "local_file"
          else
            .
          end
        )
      | .answers.delegate_answer_document.answers.app_packs |= map(
          if test("/download/[^/]+\\.gtpack$") then
            capture("/download/(?<file>[^/]+\\.gtpack)$").file as $file
            | ($demos_dir + "/" + $file)
          else
            .
          end
        )
    ' "$source_answers" > "$temp_answers"

    if ! gtc wizard --answers "$temp_answers" >/dev/null; then
        echo "Skipping $bundle_id: bundle wizard create failed" >&2
        continue
    fi

    if [ ! -f "$built_bundle" ]; then
        echo "Skipping $bundle_id: wizard did not produce $built_bundle" >&2
        continue
    fi

    cp "$built_bundle" "$target_bundle"
    echo "Created demos/${bundle_id}.gtbundle"
    packaged_any=1
done

if [ "$packaged_any" -eq 0 ]; then
    echo "No demo artifacts were packaged successfully." >&2
    exit 1
fi
