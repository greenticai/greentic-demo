#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname "$0")/.." && pwd)
CRATES_DIR="$ROOT_DIR/crates"
DEMOS_DIR="$ROOT_DIR/demos"
TMP_ROOT="${TMPDIR:-/tmp}/greentic-demo-package"
DEFAULT_PACK_ANSWERS="$TMP_ROOT/pack-update-answers.json"
LOCAL_PACK_INPUT_DIR="$TMP_ROOT/local-pack-inputs"

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
mkdir -p "$LOCAL_PACK_INPUT_DIR"
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
missing_expected=0

if [ ${#pack_dirs[@]} -eq 0 ] && [ ${#generated_pack_answers[@]} -eq 0 ]; then
    echo "No demo pack sources found under crates/. Nothing to package." >&2
    exit 1
fi

for source_pack_dir in "${pack_dirs[@]}"; do
    pack_name="$(basename "$source_pack_dir" .pack)"
    crate_dir="$(cd "$source_pack_dir/../../.." && pwd)"
    source_assets_dir="$source_pack_dir/assets"
    source_components_dir="$source_pack_dir/components"
    source_flows_dir="$source_pack_dir/flows"
    pack_answers="$DEFAULT_PACK_ANSWERS"
    create_answers="$crate_dir/gtc_pack_create_wizard_answers.json"
    flow_answers="$crate_dir/gtc_flow_wizard_answers.json"
    temp_pack_dir="$TMP_ROOT/packs/$pack_name"
    built_pack="$temp_pack_dir/dist/$pack_name.gtpack"
    target_pack="$DEMOS_DIR/$pack_name.gtpack"

    if [ -f "$crate_dir/gtc_wizard_answers.json" ]; then
        expected_pack_file="$(jq -r '
          .answers.delegate_answer_document.answers.app_pack_entries[0].reference // empty
          | select(test("(^|/)demos/[^/]+\\.gtpack$"))
          | capture("(?<file>[^/]+\\.gtpack)$").file
        ' "$crate_dir/gtc_wizard_answers.json")"
        if [ -n "$expected_pack_file" ]; then
            target_pack="$DEMOS_DIR/$expected_pack_file"
        fi
    fi

    if [ -d "$crate_dir/assets" ]; then
        source_assets_dir="$crate_dir/assets"
    fi

    if [ -d "$crate_dir/components" ]; then
        source_components_dir="$crate_dir/components"
    fi

    if [ -d "$crate_dir/flows" ]; then
        source_flows_dir="$crate_dir/flows"
    fi

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

        if [ -d "$source_pack_dir" ] && [ ! -f "$crate_dir/pack.yaml" ] && [ ! -d "$crate_dir/flows" ]; then
            # Preserve the legacy demo pack content while still creating the pack via answers.
            rsync -a --exclude 'dist/' "$source_pack_dir/" "$temp_pack_dir/"
        fi

        if [ -d "$source_assets_dir" ]; then
            mkdir -p "$temp_pack_dir/assets"
            cp -R "$source_assets_dir/." "$temp_pack_dir/assets/"
        fi

        if [ -d "$source_components_dir" ]; then
            mkdir -p "$temp_pack_dir/components"
            cp -R "$source_components_dir/." "$temp_pack_dir/components/"
        fi

        if [ -f "$flow_answers" ]; then
            if ! greentic-flow wizard "$temp_pack_dir" --answers "$flow_answers" >/dev/null; then
                echo "Skipping $pack_name: flow wizard replay failed" >&2
                continue
            fi
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

        if [ -d "$source_assets_dir" ]; then
            mkdir -p "$temp_pack_dir/assets"
            rm -rf "$temp_pack_dir/assets"
            cp -R "$source_assets_dir" "$temp_pack_dir/assets"
        fi

        if [ -d "$source_components_dir" ]; then
            mkdir -p "$temp_pack_dir/components"
            rm -rf "$temp_pack_dir/components"
            cp -R "$source_components_dir" "$temp_pack_dir/components"
        fi

        if [ -d "$source_flows_dir" ]; then
            mkdir -p "$temp_pack_dir/flows"
            rm -rf "$temp_pack_dir/flows"
            cp -R "$source_flows_dir" "$temp_pack_dir/flows"
        fi

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
    flow_answers="$crate_dir/gtc_flow_wizard_answers.json"
    pack_answers="$crate_dir/gtc_pack_wizard_answers.json"
    source_assets_dir="$crate_dir/assets"
    source_components_dir="$crate_dir/components"
    source_flows_dir="$crate_dir/flows"
    pack_dir_name="$(jq -r '.answers.pack_dir' "$create_answers" | xargs basename)"
    pack_id="$(jq -r '.answers.create_pack_id' "$create_answers")"
    pack_name="${pack_id%.pack}"
    pack_slug="${pack_dir_name%.pack}"

    if compgen -G "$crate_dir/bundle/packs/*.pack" >/dev/null; then
        continue
    fi

    if [ ! -f "$pack_answers" ]; then
        pack_answers="$DEFAULT_PACK_ANSWERS"
    fi

    temp_pack_parent="$TMP_ROOT/packs-create/$pack_name"
    temp_pack_dir="$temp_pack_parent/$pack_dir_name"
    built_pack="$temp_pack_dir/dist/$pack_dir_name.gtpack"
    target_pack="$DEMOS_DIR/$pack_slug.gtpack"

    if [ -f "$crate_dir/gtc_wizard_answers.json" ]; then
        expected_pack_file="$(jq -r '
          .answers.delegate_answer_document.answers.app_pack_entries[0].reference // empty
          | select(test("(^|/)demos/[^/]+\\.gtpack$"))
          | capture("(?<file>[^/]+\\.gtpack)$").file
        ' "$crate_dir/gtc_wizard_answers.json")"
        if [ -n "$expected_pack_file" ]; then
            target_pack="$DEMOS_DIR/$expected_pack_file"
        fi
    fi

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

    if [ -f "$flow_answers" ]; then
        if ! greentic-flow wizard "$temp_pack_dir" --answers "$flow_answers" >/dev/null; then
            echo "Skipping $pack_name: flow wizard replay failed" >&2
            continue
        fi
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
    echo "Created demos/$(basename "$target_pack")"
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

    jq --arg local_pack_dir "$LOCAL_PACK_INPUT_DIR" --arg out "$output_dir" '
      .answers.delegate_answer_document.answers.output_dir = $out
      | .answers.delegate_answer_document.answers.app_pack_entries |= map(
          if (.reference | test("/download/[^/]+\\.gtpack$")) then
            .reference as $ref
            | ($ref | capture("/download/(?<file>[^/]+\\.gtpack)$").file) as $file
            | .reference = ($local_pack_dir + "/" + $file)
            | .detected_kind = "local_file"
          else
            .
          end
        )
      | .answers.delegate_answer_document.answers.app_packs |= map(
          if test("/download/[^/]+\\.gtpack$") then
            capture("/download/(?<file>[^/]+\\.gtpack)$").file as $file
            | ($local_pack_dir + "/" + $file)
          else
            .
          end
        )
    ' "$source_answers" > "$temp_answers"

    if ! gtc wizard --answers "$temp_answers" >/dev/null; then
        echo "Skipping $bundle_id: bundle wizard create failed" >&2
        continue
    fi

    # Some create-answer documents produce a workspace (bundle.yaml + providers/packs)
    # but do not emit dist/*.gtbundle directly. Build explicitly in that case.
    if [ ! -f "$built_bundle" ] && [ -f "$output_dir/bundle.yaml" ]; then
        if ! (
            cd "$output_dir"
            greentic-bundle build >/dev/null
        ); then
            echo "Skipping $bundle_id: bundle build failed after wizard create" >&2
            continue
        fi
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

# Bundle creation may consume local pack inputs. Use temp copies instead of demos/*.gtpack.
find "$LOCAL_PACK_INPUT_DIR" -mindepth 1 -maxdepth 1 -name '*.gtpack' -delete
find "$DEMOS_DIR" -mindepth 1 -maxdepth 1 -name '*.gtpack' -exec cp {} "$LOCAL_PACK_INPUT_DIR/" \;

for source_answers in "${bundle_answers[@]}"; do
    demo_basename="$(basename "$source_answers" -create-answers.json)"
    bundle_id="$(jq -r '.answers.delegate_answer_document.answers.bundle_id' "$source_answers")"
    expected_bundle="$DEMOS_DIR/${bundle_id}.gtbundle"
    expected_pack="$(jq -r '
      .answers.delegate_answer_document.answers.app_pack_entries[0].reference // empty
      | capture("(?<file>[^/]+\\.gtpack)$").file? // empty
    ' "$source_answers")"

    if [ ! -f "$expected_bundle" ]; then
        echo "Missing expected bundle for $demo_basename: $expected_bundle" >&2
        missing_expected=1
    fi

    if [ -n "$expected_pack" ] && [ ! -f "$DEMOS_DIR/$expected_pack" ]; then
        echo "Missing expected pack for $demo_basename: $DEMOS_DIR/$expected_pack" >&2
        missing_expected=1
    fi
done

if [ "$missing_expected" -ne 0 ]; then
    echo "One or more expected demo artifacts were not produced." >&2
    exit 1
fi
