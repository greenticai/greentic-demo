#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname "$0")/../../../../../.." && pwd)

create_answers="$ROOT_DIR/demos/invoice-approval-create-answers.json"
setup_answers="$ROOT_DIR/demos/invoice-approval-setup-answers.json"
pack_file="$ROOT_DIR/demos/invoice-approval.gtpack"

jq -e '.answers.delegate_answer_document.answers.output_dir == "./invoice-approval-bundle"' "$create_answers" >/dev/null
jq -e '.answers.delegate_answer_document.answers.bundle_id == "invoice-approval-bundle"' "$create_answers" >/dev/null
jq -e '.answers.delegate_answer_document.answers.bundle_name == "invoice-approval-bundle"' "$create_answers" >/dev/null
jq -e '.answers.delegate_answer_document.answers.app_pack_entries[0].reference == "https://github.com/greenticai/greentic-demo/releases/latest/download/invoice-approval.gtpack"' "$create_answers" >/dev/null

jq -e '.tenant == "demo"' "$setup_answers" >/dev/null
jq -e '.setup_answers."messaging-webchat-gui".public_base_url == "http://localhost:8080"' "$setup_answers" >/dev/null

[ -f "$pack_file" ]

echo "invoice-approval validation passed"
