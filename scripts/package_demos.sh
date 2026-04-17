#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname "$0")/.." && pwd)
CRATES_DIR="$ROOT_DIR/crates"
DEMOS_DIR="$ROOT_DIR/demos"
TMP_ROOT="${TMPDIR:-/tmp}/greentic-demo-package"
DEFAULT_PACK_ANSWERS="$TMP_ROOT/pack-update-answers.json"
LOCAL_PACK_INPUT_DIR="$TMP_ROOT/local-pack-inputs"
# Max seconds per wizard/setup command before it is killed.
WIZARD_TIMEOUT="${WIZARD_TIMEOUT:-180}"

run_with_timeout() {
    local seconds="$1"
    shift

    if command -v timeout >/dev/null 2>&1; then
        timeout "$seconds" "$@"
        return
    fi

    if command -v gtimeout >/dev/null 2>&1; then
        gtimeout "$seconds" "$@"
        return
    fi

    "$@"
}

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

# Close stdin so wizard/setup commands that attempt interactive prompts
# (e.g. secret reads) fail immediately instead of hanging in CI.
exec < /dev/null

mkdir -p "$CRATES_DIR" "$DEMOS_DIR"
rm -rf "$TMP_ROOT"
mkdir -p "$TMP_ROOT"
mkdir -p "$LOCAL_PACK_INPUT_DIR"
# Seed LOCAL_PACK_INPUT_DIR with committed packs before cleanup so that
# pre-built packs without rebuild sources (e.g. cloud-deploy-demo-app.gtpack)
# remain available for bundle creation.
find "$DEMOS_DIR" -mindepth 1 -maxdepth 1 -name '*.gtpack' -exec cp {} "$LOCAL_PACK_INPUT_DIR/" \;
find "$DEMOS_DIR" -mindepth 1 -maxdepth 1 -name '*.gtbundle' -exec rm -rf {} +
find "$DEMOS_DIR" -mindepth 1 -maxdepth 1 -name '*.gtpack' -delete

run_bundle_build() {
    local root="$1"
    local output="$2"

    if command -v greentic-setup >/dev/null 2>&1; then
        run_with_timeout "$WIZARD_TIMEOUT" greentic-setup bundle build --bundle "$root" --out "$output" >/dev/null
    else
        (
            cd "$ROOT_DIR"
            run_with_timeout "$WIZARD_TIMEOUT" cargo run -q -p greentic-setup --bin greentic-setup -- bundle build --bundle "$root" --out "$output" >/dev/null
        )
    fi
}

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
    "run_build": false,
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

# Helper: resolve wizard answer files from build-answer.json or legacy per-file layout.
# Sets: _wizard _pack_create _pack _flow (paths to extracted or original files)
resolve_answers() {
    local crate_dir="$1"
    local build_answer="$crate_dir/build-answer.json"
    _wizard="" _pack_create="" _pack="" _flow=""
    if [ -f "$build_answer" ]; then
        local extract_dir="$TMP_ROOT/extracted-answers/$(basename "$crate_dir")"
        mkdir -p "$extract_dir"
        for section in wizard pack_create pack flow; do
            local val
            val=$(jq -r ".$section // empty" "$build_answer") || true
            if [ -n "$val" ] && [ "$val" != "null" ]; then
                jq ".$section" "$build_answer" > "$extract_dir/$section.json"
                eval "_$section=\"$extract_dir/$section.json\""
            fi
        done
    else
        [ -f "$crate_dir/gtc_wizard_answers.json" ] && _wizard="$crate_dir/gtc_wizard_answers.json"
        [ -f "$crate_dir/gtc_pack_create_wizard_answers.json" ] && _pack_create="$crate_dir/gtc_pack_create_wizard_answers.json" || true
        [ -f "$crate_dir/pack_answers.json" ] && [ -z "$_pack_create" ] && _pack_create="$crate_dir/pack_answers.json" || true
        [ -f "$crate_dir/gtc_pack_wizard_answers.json" ] && _pack="$crate_dir/gtc_pack_wizard_answers.json" || true
        [ -f "$crate_dir/gtc_flow_wizard_answers.json" ] && _flow="$crate_dir/gtc_flow_wizard_answers.json" || true
    fi
}

pack_dirs=("$CRATES_DIR"/*/bundle/packs/*.pack)
# Discover pack create answers from build-answer.json OR legacy per-file layout.
generated_pack_answers=()
for _cdir in "$CRATES_DIR"/*/; do
    if [ -f "$_cdir/build-answer.json" ] && jq -e '.pack_create' "$_cdir/build-answer.json" >/dev/null 2>&1; then
        generated_pack_answers+=("$_cdir/build-answer.json")
    elif [ -f "$_cdir/gtc_pack_create_wizard_answers.json" ]; then
        generated_pack_answers+=("$_cdir/gtc_pack_create_wizard_answers.json")
    elif [ -f "$_cdir/pack_answers.json" ]; then
        generated_pack_answers+=("$_cdir/pack_answers.json")
    fi
done
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
    resolve_answers "$crate_dir"
    pack_answers="${_pack:-$DEFAULT_PACK_ANSWERS}"
    create_answers="${_pack_create:-}"
    flow_answers="${_flow:-}"
    temp_pack_dir="$TMP_ROOT/packs/$pack_name"
    pack_dir_basename="$(basename "$source_pack_dir")"
    built_pack="$temp_pack_dir/dist/$pack_dir_basename.gtpack"
    target_pack="$DEMOS_DIR/$pack_name.gtpack"

    # If a pre-built pack was committed in demos/ (seeded into LOCAL_PACK_INPUT_DIR),
    # skip the crate-source rebuild and use the committed pack directly.
    if [ -f "$LOCAL_PACK_INPUT_DIR/$pack_name.gtpack" ]; then
        cp "$LOCAL_PACK_INPUT_DIR/$pack_name.gtpack" "$DEMOS_DIR/$pack_name.gtpack"
        echo "Using committed demos/$pack_name.gtpack (skipping crate rebuild)"
        packaged_any=1
        continue
    fi

    if [ -n "$_wizard" ]; then
        expected_pack_file="$(jq -r '
          .answers.delegate_answer_document.answers.app_pack_entries[0].reference // empty
          | select(test("(^|/)demos/[^/]+\\.gtpack$"))
          | capture("(?<file>[^/]+\\.gtpack)$").file
        ' "$_wizard")"
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

    if [ -n "$create_answers" ]; then
        temp_pack_parent="$TMP_ROOT/packs-create/$pack_name"
        temp_pack_dir="$temp_pack_parent/$pack_name.pack"
        built_pack="$temp_pack_dir/dist/$pack_name.pack.gtpack"

        rm -rf "$temp_pack_parent"
        mkdir -p "$temp_pack_parent"

        if ! (
            cd "$temp_pack_parent"
            run_with_timeout "$WIZARD_TIMEOUT" greentic-pack wizard apply --answers "$create_answers" >/dev/null
        ); then
            echo "Skipping $pack_name: pack scaffold wizard create failed" >&2
            continue
        fi

        if [ -d "$source_pack_dir" ] && [ ! -f "$crate_dir/pack.yaml" ] && [ ! -d "$crate_dir/flows" ]; then
            # Preserve the legacy demo pack content while still creating the pack via answers.
            # Exclude dist, resolve files (scaffold wizard creates OCI-resolved ones), and components.
            rsync -a --exclude 'dist/' --exclude '*.resolve.json' --exclude '*.resolve.summary.json' --exclude 'components/' "$source_pack_dir/" "$temp_pack_dir/"
        fi

        if [ -d "$source_assets_dir" ]; then
            mkdir -p "$temp_pack_dir/assets"
            cp -R "$source_assets_dir/." "$temp_pack_dir/assets/"
        fi

        # Components are resolved from OCI by the scaffold wizard; skip local copy
        # to avoid hash conflicts between local WASM and OCI-resolved artifacts.

        if [ -f "$flow_answers" ]; then
            if ! run_with_timeout "$WIZARD_TIMEOUT" greentic-flow wizard "$temp_pack_dir" --answers "$flow_answers" >/dev/null; then
                echo "Skipping $pack_name: flow wizard replay failed" >&2
                continue
            fi
        fi

        if ! (
            cd "$temp_pack_dir"
            run_with_timeout "$WIZARD_TIMEOUT" greentic-pack wizard apply --answers "$pack_answers" >/dev/null
        ); then
            echo "Skipping $pack_name: pack wizard update failed after scaffold replay" >&2
            continue
        fi

        # Restore committed resolve/summary files — the wizard's internal resolve
        # may overwrite them with empty entries when components are unavailable in CI.
        if [ -d "$source_flows_dir" ]; then
            find "$source_flows_dir" \( -name '*.resolve.json' -o -name '*.resolve.summary.json' \) 2>/dev/null |
            while read -r rf; do
                cp "$rf" "$temp_pack_dir/flows/$(basename "$rf")" 2>/dev/null || true
            done
        fi

        # Build separately after resolve files are restored.
        if ! (cd "$temp_pack_dir" && run_with_timeout "$WIZARD_TIMEOUT" greentic-pack build --in . >/dev/null); then
            echo "Skipping $pack_name: pack build failed after scaffold replay" >&2
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
            run_with_timeout "$WIZARD_TIMEOUT" greentic-pack wizard apply --answers "$pack_answers" >/dev/null
        ); then
            echo "Skipping $pack_name: pack wizard update failed" >&2
            continue
        fi

        # Restore committed resolve/summary files — the wizard's internal resolve
        # may overwrite them with empty entries when components are unavailable in CI.
        find "$source_flows_dir" \( -name '*.resolve.json' -o -name '*.resolve.summary.json' \) 2>/dev/null |
        while read -r rf; do
            cp "$rf" "$temp_pack_dir/flows/$(basename "$rf")" 2>/dev/null || true
        done

        # Build separately after resolve files are restored.
        if ! (cd "$temp_pack_dir" && run_with_timeout "$WIZARD_TIMEOUT" greentic-pack build --in . >/dev/null); then
            echo "Skipping $pack_name: pack build failed" >&2
            continue
        fi
    fi

    if [ ! -f "$built_pack" ]; then
        dist_packs=("$temp_pack_dir"/dist/*.gtpack)
        if [ ${#dist_packs[@]} -eq 1 ]; then
            built_pack="${dist_packs[0]}"
        else
            echo "Skipping $pack_name: wizard did not produce $built_pack" >&2
            continue
        fi
    fi

    cp "$built_pack" "$target_pack"
    echo "Created demos/$pack_name.gtpack"
    packaged_any=1
done

for _gen_source in "${generated_pack_answers[@]}"; do
    crate_dir="$(cd "$(dirname "$_gen_source")" && pwd)"
    resolve_answers "$crate_dir"
    create_answers="${_pack_create:-}"
    [ -z "$create_answers" ] && continue
    pack_build_script="$crate_dir/build_pack.sh"
    flow_answers="${_flow:-}"
    pack_answers="${_pack:-}"
    source_assets_dir="$crate_dir/assets"
    source_components_dir="$crate_dir/components"
    source_flows_dir="$crate_dir/flows"
    source_pack_overlay_dir="$crate_dir/generated-pack"
    pack_dir_name="$(jq -r '.answers.pack_dir' "$create_answers" | xargs basename)"
    pack_id="$(jq -r '.answers.create_pack_id' "$create_answers")"
    pack_name="${pack_id%.pack}"
    pack_slug="${pack_dir_name%.pack}"

    if compgen -G "$crate_dir/bundle/packs/*.pack" >/dev/null; then
        continue
    fi

    if [ -z "$pack_answers" ] || [ ! -f "$pack_answers" ] || [ "$pack_answers" = "$create_answers" ]; then
        pack_answers="$DEFAULT_PACK_ANSWERS"
    fi

    temp_pack_parent="$TMP_ROOT/packs-create/$pack_name"
    temp_pack_dir="$temp_pack_parent/$pack_dir_name"
    built_pack="$temp_pack_dir/dist/$pack_dir_name.gtpack"
    target_pack="$DEMOS_DIR/$pack_slug.gtpack"

    # Resolve expected pack filename from wizard answers (if available).
    _seeded_name="$pack_slug.gtpack"
    if [ -n "$_wizard" ]; then
        _expected="$(jq -r '
          .answers.delegate_answer_document.answers.app_pack_entries[0].reference // empty
          | select(test("(^|/)demos/[^/]+\\.gtpack$"))
          | capture("(?<file>[^/]+\\.gtpack)$").file
        ' "$_wizard")"
        if [ -n "$_expected" ]; then
            _seeded_name="$_expected"
        fi
    fi

    if [ -n "$_wizard" ]; then
        expected_pack_file="$(jq -r '
          .answers.delegate_answer_document.answers.app_pack_entries[0].reference // empty
          | select(test("(^|/)demos/[^/]+\\.gtpack$"))
          | capture("(?<file>[^/]+\\.gtpack)$").file
        ' "$_wizard")"
        if [ -n "$expected_pack_file" ]; then
            target_pack="$DEMOS_DIR/$expected_pack_file"
        fi
    fi

    if [ -x "$pack_build_script" ]; then
        if ! "$pack_build_script" "$target_pack" >/dev/null; then
            echo "Skipping $pack_name: custom pack build script failed" >&2
            continue
        fi
        echo "Created demos/$(basename "$target_pack")"
        packaged_any=1
        continue
    fi

    # If a pre-built pack was committed in demos/, skip the crate-source rebuild.
    if [ -f "$LOCAL_PACK_INPUT_DIR/$_seeded_name" ]; then
        cp "$LOCAL_PACK_INPUT_DIR/$_seeded_name" "$DEMOS_DIR/$_seeded_name"
        echo "Using committed demos/$_seeded_name (skipping crate rebuild)"
        packaged_any=1
        continue
    fi

    rm -rf "$temp_pack_parent"
    mkdir -p "$temp_pack_parent"

    if ! (
        cd "$temp_pack_parent"
        run_with_timeout "$WIZARD_TIMEOUT" greentic-pack wizard apply --answers "$create_answers" >/dev/null
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

    if [ -n "$flow_answers" ] && [ -f "$flow_answers" ]; then
        if ! run_with_timeout "$WIZARD_TIMEOUT" greentic-flow wizard "$temp_pack_dir" --answers "$flow_answers" >/dev/null; then
            echo "Skipping $pack_name: flow wizard replay failed" >&2
            continue
        fi
    fi

    if [ -d "$source_pack_overlay_dir" ]; then
        rsync -a --exclude 'dist/' "$source_pack_overlay_dir/." "$temp_pack_dir/"
    fi

    if [ -d "$source_pack_overlay_dir/flows" ]; then
        source_flows_dir="$source_pack_overlay_dir/flows"
    fi

    if ! (
        cd "$temp_pack_dir"
        run_with_timeout "$WIZARD_TIMEOUT" greentic-pack wizard apply --answers "$pack_answers" >/dev/null
    ); then
        echo "Skipping $pack_name: pack wizard update failed after scaffold replay" >&2
        continue
    fi

    # Restore committed resolve/summary files — the wizard's internal resolve
    # may overwrite them with empty entries when components are unavailable in CI.
    if [ -d "$source_flows_dir" ]; then
        find "$source_flows_dir" \( -name '*.resolve.json' -o -name '*.resolve.summary.json' \) 2>/dev/null |
        while read -r rf; do
            cp "$rf" "$temp_pack_dir/flows/$(basename "$rf")" 2>/dev/null || true
        done
    fi

    # Build separately after resolve files are restored.
    if ! (cd "$temp_pack_dir" && run_with_timeout "$WIZARD_TIMEOUT" greentic-pack build --in . >/dev/null); then
        echo "Skipping $pack_name: pack build failed after scaffold replay" >&2
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
    setup_answers="$DEMOS_DIR/${demo_basename}-setup-answers.json"
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

    if ! run_with_timeout "$WIZARD_TIMEOUT" gtc wizard --answers "$temp_answers" >/dev/null; then
        echo "Skipping $bundle_id: bundle wizard create failed" >&2
        continue
    fi

    if [ -f "$setup_answers" ]; then
        if ! run_with_timeout "$WIZARD_TIMEOUT" gtc setup --answers "$setup_answers" "$output_dir" >/dev/null 2>&1; then
            echo "Warning: $bundle_id: bundle setup failed (missing secrets?), attempting build anyway" >&2
        fi
    fi

    # Some create-answer documents produce a workspace (bundle.yaml + providers/packs)
    # but do not emit dist/*.gtbundle directly. Build explicitly in that case.
    if [ ! -f "$built_bundle" ] && [ -f "$output_dir/bundle.yaml" ]; then
        if ! run_bundle_build "$output_dir" "$built_bundle"; then
            echo "Skipping $bundle_id: bundle build failed after wizard create" >&2
            continue
        fi
    fi

    if [ ! -f "$built_bundle" ]; then
        echo "Skipping $bundle_id: wizard did not produce $built_bundle" >&2
        continue
    fi

    rm -rf "$target_bundle"
    cp "$built_bundle" "$target_bundle"
    echo "Created demos/${bundle_id}.gtbundle"
    packaged_any=1
done

if [ "$packaged_any" -eq 0 ]; then
    echo "No demo artifacts were packaged successfully." >&2
    exit 1
fi

# Restore pre-seeded packs that weren't rebuilt (e.g. cloud-deploy-demo-app.gtpack).
for seeded in "$LOCAL_PACK_INPUT_DIR"/*.gtpack; do
    [ -f "$seeded" ] || continue
    seeded_name="$(basename "$seeded")"
    if [ ! -f "$DEMOS_DIR/$seeded_name" ]; then
        cp "$seeded" "$DEMOS_DIR/$seeded_name"
    fi
done

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
