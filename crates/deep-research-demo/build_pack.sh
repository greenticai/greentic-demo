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

# Step 1: scaffold pack dir + resolve components (uses pack_answers.json for
# component metadata / wizard answers; the flow body is overwritten below).
jq --arg pack_dir "$PACK_DIR" '
  .answers.pack_dir = $pack_dir
' "$ROOT_DIR/pack_answers.json" > "$ANSWERS_PATH"

(
    cd "$ROOT_DIR"
    PATH="$FLOW_BIN_DIR:$(dirname "$GREENTIC_PACK_BIN"):$PATH" "$GREENTIC_PACK_BIN" wizard apply --answers "$ANSWERS_PATH" >/dev/null
)

# Step 2: overwrite the wizard-generated flow with the hand-written deep
# research flow. The wizard output is broken for this demo: update-step
# actions with `in_map` template bindings are silently ignored, producing a
# linear flow with no user-question propagation or LLM input mapping. We
# write the full flow inline here instead.
cat > "$PACK_DIR/flows/main.ygtc" <<'EOF'
id: main
type: messaging
start: main_menu
parameters: {}
tags: []
schema_version: 2
entrypoints: {}
nodes:
  main_menu:
    component.exec:
      component: oci://ghcr.io/greenticai/components/component-adaptive-card:latest
      operation: card
      input:
        card_source: asset
        card_spec:
          asset_path: assets/cards/main_menu.json
        mode: renderAndValidate
        validation_mode: warn
        node_id: main_menu
        payload: {}
    routing:
      - condition: response.action == "create_research_plan"
        to: research_planner
      - condition: response.action == "start_research_analysis"
        to: research_analyst
      - out: true

  research_planner:
    component.exec:
      component: oci://ghcr.io/greenticai/component/component-llm-openai:latest
      operation: handle_message
      input:
        config:
          provider: ollama
          base_url: http://127.0.0.1:11434/v1
          default_model: llama3:8b
        input:
          messages:
            - role: system
              content: |
                You are a Research Planner Agent. Turn the user's research goal into a focused, practical research plan with clear scope, key questions, assumptions, risks, sources, and sequenced next steps.
            - role: user
              content: "{{entry.input.metadata.user_question}}"
    routing:
      - to: show_research_plan

  show_research_plan:
    component.exec:
      component: oci://ghcr.io/greenticai/components/component-adaptive-card:latest
      operation: card
      input:
        card_source: asset
        card_spec:
          asset_path: assets/cards/research_plan.json
        mode: renderAndValidate
        validation_mode: warn
        node_id: show_research_plan
        payload:
          user_question: "{{entry.input.metadata.user_question}}"
          generatingVisible: false
          researchPlanVisible: true
          planner_output: "{{prev.completion}}"
    routing:
      - out: true

  research_analyst:
    component.exec:
      component: oci://ghcr.io/greenticai/component/component-llm-openai:latest
      operation: handle_message
      input:
        config:
          provider: ollama
          base_url: http://127.0.0.1:11434/v1
          default_model: llama3:8b
        input:
          messages:
            - role: system
              content: |
                You are a Research Execution Agent. Execute the supplied plan carefully, use reliable evidence, separate facts from assumptions, and produce a concise but useful final report with summary, findings, evidence, contradictions, risks, and a confidence score.
            - role: user
              content: "Research question: {{entry.input.metadata.user_question}}\n\nResearch plan:\n{{entry.input.metadata.planner_output}}"
    routing:
      - to: show_final_report

  show_final_report:
    component.exec:
      component: oci://ghcr.io/greenticai/components/component-adaptive-card:latest
      operation: card
      input:
        card_source: asset
        card_spec:
          asset_path: assets/cards/final_report.json
        mode: renderAndValidate
        validation_mode: warn
        node_id: show_final_report
        payload:
          user_question: "{{entry.input.metadata.user_question}}"
          processingVisible: false
          finalReportVisible: true
          report_output: "{{prev.completion}}"
    routing:
      - out: true
EOF

# Step 3: stage assets, resolve component lock, build the pack archive.
cp -R "$ROOT_DIR/assets" "$PACK_DIR/"

(
    cd "$PACK_DIR"
    PATH="$FLOW_BIN_DIR:$(dirname "$GREENTIC_PACK_BIN"):$PATH" "$GREENTIC_PACK_BIN" resolve --in . >/dev/null
    PATH="$FLOW_BIN_DIR:$(dirname "$GREENTIC_PACK_BIN"):$PATH" "$GREENTIC_PACK_BIN" build --in . >/dev/null
)

mkdir -p "$(dirname "$OUT_PATH")"
cp "$PACK_DIR/dist/deep-research-demo.pack.gtpack" "$OUT_PATH"
echo "created $OUT_PATH"
