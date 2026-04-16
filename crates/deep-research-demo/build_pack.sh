#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname "$0")" && pwd)
REPO_ROOT=$(cd -- "$ROOT_DIR/../.." && pwd)
OUT_PATH="${1:-$REPO_ROOT/demos/deep-research-demo.gtpack}"
TMP_DIR=$(mktemp -d "${TMPDIR:-/tmp}/deep-research-pack.XXXXXX")
PACK_DIR="$TMP_DIR/deep-research-demo.pack"

cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

cd "$TMP_DIR"
greentic-pack wizard apply --answers "$ROOT_DIR/pack_answers.json" >/dev/null
cp -R "$ROOT_DIR/assets" "$PACK_DIR/"

cat > "$PACK_DIR/pack.yaml" <<'EOF'
pack_id: deep-research-demo
version: 0.1.0
kind: application
publisher: Greentic
display_name: Deep Research Demo
components: []
dependencies: []
flows:
- id: on_message
  file: flows/main.ygtc
  tags:
  - default
  - messaging
  entrypoints:
  - default
assets:
- path: assets/cards/main_menu.json
- path: assets/cards/research_plan.json
- path: assets/cards/final_report.json
EOF

mkdir -p "$PACK_DIR/flows"

cat > "$PACK_DIR/flows/main.ygtc" <<'EOF'
id: on_message
type: messaging
start: main_menu
parameters: {}
tags: []
schema_version: 2
entrypoints: {}
nodes:
  main_menu:
    card:
      card_source: asset
      mode: renderAndValidate
      validation_mode: warn
      node_id: main_menu
      card_spec:
        asset_path: assets/cards/main_menu.json
      payload:
        user_question: "{{state.user_question}}"
      session: {}
      state: {}
    routing:
      - condition: response.nextCardId == "research_planner"
        to: research_planner
      - out: true

  research_planner:
    component.exec:
      component: oci://ghcr.io/greenticai/component/component-llm-openai:latest
      operation: invoke
      input:
        config:
          provider: ollama
          base_url: http://127.0.0.1:11434/v1
          default_model: llama3.2
        input:
          model: llama3.2
          messages:
            - role: system
              content: |
                You are a research planner.
                Return a concise research plan for the user's question.
                Use plain text with clear sections for sub-questions, required sources, and report structure.
            - role: user
              content: "{{state.user_question}}"
    routing:
      - to: show_research_plan

  show_research_plan:
    card:
      card_source: asset
      mode: renderAndValidate
      validation_mode: warn
      node_id: show_research_plan
      card_spec:
        asset_path: assets/cards/research_plan.json
      payload:
        user_question: "{{state.user_question}}"
        generatingVisible: false
        researchPlanVisible: true
        planner_output: "{{node.research_planner.result.structured_content.completion}}"
      session: {}
      state: {}
    routing:
      - condition: response.nextCardId == "research_analyst"
        to: research_analyst
      - condition: response.nextCardId == "main_menu"
        to: reset_to_main_menu
      - out: true

  research_analyst:
    component.exec:
      component: oci://ghcr.io/greenticai/component/component-llm-openai:latest
      operation: invoke
      input:
        config:
          provider: ollama
          base_url: http://127.0.0.1:11434/v1
          default_model: llama3.2
        input:
          model: llama3.2
          messages:
            - role: system
              content: |
                You are a research analyst.
                Produce a concise but useful report that includes summary, findings, evidence, contradictions, risks, and a confidence score.
            - role: user
              content: "Question: {{state.user_question}}"
            - role: user
              content: "Research plan: {{node.research_planner.result.structured_content.completion}}"
    routing:
      - to: show_final_report

  show_final_report:
    card:
      card_source: asset
      mode: renderAndValidate
      validation_mode: warn
      node_id: show_final_report
      card_spec:
        asset_path: assets/cards/final_report.json
      payload:
        user_question: "{{state.user_question}}"
        processingVisible: false
        finalReportVisible: true
        report_output: "{{node.research_analyst.result.structured_content.completion}}"
      session: {}
      state: {}
    routing:
      - condition: response.nextCardId == "main_menu"
        to: reset_to_main_menu
      - out: true

  reset_to_main_menu:
    card:
      card_source: asset
      mode: renderAndValidate
      validation_mode: warn
      node_id: reset_to_main_menu
      card_spec:
        asset_path: assets/cards/main_menu.json
      payload: {}
      session: {}
      state: {}
    routing:
      - condition: response.nextCardId == "research_planner"
        to: research_planner
      - out: true
EOF

cat > "$PACK_DIR/flows/main.ygtc.resolve.json" <<'EOF'
{
  "schema_version": 1,
  "flow": "main.ygtc",
  "nodes": {
    "main_menu": {
      "source": {
        "kind": "oci",
        "ref": "oci://ghcr.io/greenticai/components/component-adaptive-card:latest"
      },
      "digest": "sha256:35eead0b22993b3d8a8bfe50e82d90a8b7bd9b2827b16e0f87fee86a0f2ac91c"
    },
    "research_planner": {
      "source": {
        "kind": "oci",
        "ref": "oci://ghcr.io/greenticai/component/component-llm-openai:latest"
      },
      "digest": "sha256:a5fef28217d8fcc5f4ab30ce40710d399131383b11c212abdeeb1b484b65e4f7"
    },
    "show_research_plan": {
      "source": {
        "kind": "oci",
        "ref": "oci://ghcr.io/greenticai/components/component-adaptive-card:latest"
      },
      "digest": "sha256:35eead0b22993b3d8a8bfe50e82d90a8b7bd9b2827b16e0f87fee86a0f2ac91c"
    },
    "research_analyst": {
      "source": {
        "kind": "oci",
        "ref": "oci://ghcr.io/greenticai/component/component-llm-openai:latest"
      },
      "digest": "sha256:a5fef28217d8fcc5f4ab30ce40710d399131383b11c212abdeeb1b484b65e4f7"
    },
    "show_final_report": {
      "source": {
        "kind": "oci",
        "ref": "oci://ghcr.io/greenticai/components/component-adaptive-card:latest"
      },
      "digest": "sha256:35eead0b22993b3d8a8bfe50e82d90a8b7bd9b2827b16e0f87fee86a0f2ac91c"
    },
    "reset_to_main_menu": {
      "source": {
        "kind": "oci",
        "ref": "oci://ghcr.io/greenticai/components/component-adaptive-card:latest"
      },
      "digest": "sha256:35eead0b22993b3d8a8bfe50e82d90a8b7bd9b2827b16e0f87fee86a0f2ac91c"
    }
  }
}
EOF

cat > "$PACK_DIR/flows/main.ygtc.resolve.summary.json" <<'EOF'
{
  "schema_version": 1,
  "flow": "main.ygtc",
  "nodes": {
    "main_menu": {
      "component_id": "ai.greentic.component-adaptive-card",
      "source": {
        "kind": "oci",
        "ref": "oci://ghcr.io/greenticai/components/component-adaptive-card:latest"
      },
      "digest": "sha256:35eead0b22993b3d8a8bfe50e82d90a8b7bd9b2827b16e0f87fee86a0f2ac91c",
      "manifest": {
        "world": "greentic:component/component@0.6.0",
        "version": "0.1.25"
      }
    },
    "research_planner": {
      "component_id": "ai.greentic.component-llm-openai",
      "source": {
        "kind": "oci",
        "ref": "oci://ghcr.io/greenticai/component/component-llm-openai:latest"
      },
      "digest": "sha256:a5fef28217d8fcc5f4ab30ce40710d399131383b11c212abdeeb1b484b65e4f7",
      "manifest": {
        "world": "greentic:component/component@0.6.0",
        "version": "0.1.0"
      }
    },
    "show_research_plan": {
      "component_id": "ai.greentic.component-adaptive-card",
      "source": {
        "kind": "oci",
        "ref": "oci://ghcr.io/greenticai/components/component-adaptive-card:latest"
      },
      "digest": "sha256:35eead0b22993b3d8a8bfe50e82d90a8b7bd9b2827b16e0f87fee86a0f2ac91c",
      "manifest": {
        "world": "greentic:component/component@0.6.0",
        "version": "0.1.25"
      }
    },
    "research_analyst": {
      "component_id": "ai.greentic.component-llm-openai",
      "source": {
        "kind": "oci",
        "ref": "oci://ghcr.io/greenticai/component/component-llm-openai:latest"
      },
      "digest": "sha256:a5fef28217d8fcc5f4ab30ce40710d399131383b11c212abdeeb1b484b65e4f7",
      "manifest": {
        "world": "greentic:component/component@0.6.0",
        "version": "0.1.0"
      }
    },
    "show_final_report": {
      "component_id": "ai.greentic.component-adaptive-card",
      "source": {
        "kind": "oci",
        "ref": "oci://ghcr.io/greenticai/components/component-adaptive-card:latest"
      },
      "digest": "sha256:35eead0b22993b3d8a8bfe50e82d90a8b7bd9b2827b16e0f87fee86a0f2ac91c",
      "manifest": {
        "world": "greentic:component/component@0.6.0",
        "version": "0.1.25"
      }
    },
    "reset_to_main_menu": {
      "component_id": "ai.greentic.component-adaptive-card",
      "source": {
        "kind": "oci",
        "ref": "oci://ghcr.io/greenticai/components/component-adaptive-card:latest"
      },
      "digest": "sha256:35eead0b22993b3d8a8bfe50e82d90a8b7bd9b2827b16e0f87fee86a0f2ac91c",
      "manifest": {
        "world": "greentic:component/component@0.6.0",
        "version": "0.1.25"
      }
    }
  }
}
EOF

(
    cd "$PACK_DIR"
    greentic-pack build --in . >/dev/null
)

mkdir -p "$(dirname "$OUT_PATH")"
cp "$PACK_DIR/dist/deep-research-demo.pack.gtpack" "$OUT_PATH"
echo "created $OUT_PATH"
