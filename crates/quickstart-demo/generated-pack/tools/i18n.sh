#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

I18N_DIR="$ROOT_DIR/assets/i18n"
LOCALES_FILE="$I18N_DIR/locales.json"
EN_FILE="$I18N_DIR/en.json"
MODE="${1:-all}"
LOCALE="${LOCALE:-en}"
AUTH_MODE="${AUTH_MODE:-auto}"
TRANSLATOR_BIN="${TRANSLATOR_BIN:-greentic-i18n-translator}"
CODEX_SKIP_GIT_REPO_CHECK="${CODEX_SKIP_GIT_REPO_CHECK:-1}"

fail() {
  echo "error: $*" >&2
  exit 1
}

info() {
  echo "info: $*"
}

usage() {
  cat <<'USAGE'
Usage: tools/i18n.sh [all|translate|validate|status]

Environment overrides:
  LOCALE=...          Locale for translator runtime messages (default: en)
  AUTH_MODE=...       Auth mode for translate (default: auto)
  TRANSLATOR_BIN=...  Translator command (default: greentic-i18n-translator)
  CODEX_SKIP_GIT_REPO_CHECK=0|1  Add --skip-git-repo-check to codex exec (default: 1)
USAGE
}

require_files() {
  [[ -f "$LOCALES_FILE" ]] || fail "missing $LOCALES_FILE"
  [[ -f "$EN_FILE" ]] || fail "missing $EN_FILE"
}

resolve_translator_bin() {
  if command -v "$TRANSLATOR_BIN" >/dev/null 2>&1; then
    return 0
  fi
  fail "translator binary not found on PATH: $TRANSLATOR_BIN"
}

load_locales() {
  LOCALES=()
  while IFS= read -r line; do
    LOCALES+=("$line")
  done < <(python3 - <<'PY' "$LOCALES_FILE"
import json, sys
for item in json.load(open(sys.argv[1], encoding="utf-8")):
    print(item)
PY
)
  LOCALES_CSV="$(IFS=,; echo "${LOCALES[*]}")"
}

ensure_locale_files() {
  for locale in "${LOCALES[@]}"; do
    file="$I18N_DIR/$locale.json"
    if [[ ! -f "$file" ]]; then
      mkdir -p "$(dirname "$file")"
      printf "{\n}\n" > "$file"
    fi
  done
}

setup_codex_wrapper_if_needed() {
  if [[ "$CODEX_SKIP_GIT_REPO_CHECK" != "1" ]]; then
    return 0
  fi
  if ! command -v codex >/dev/null 2>&1; then
    return 0
  fi
  local real_codex
  real_codex="$(command -v codex)"
  local wrapper_dir
  wrapper_dir="$(mktemp -d)"
  cat > "$wrapper_dir/codex" <<EOF
#!/usr/bin/env bash
set -euo pipefail
if [[ "\${1:-}" == "exec" ]]; then
  shift
  exec "$real_codex" exec --skip-git-repo-check "\$@"
fi
exec "$real_codex" "\$@"
EOF
  chmod +x "$wrapper_dir/codex"
  export PATH="$wrapper_dir:$PATH"
}

run_translate() {
  info "running translate for $(echo "$LOCALES_CSV" | tr ',' ' ' | wc -w) locales from $EN_FILE"
  setup_codex_wrapper_if_needed
  "$TRANSLATOR_BIN" --locale "$LOCALE" \
    translate --langs "$LOCALES_CSV" --en "$EN_FILE" --auth-mode "$AUTH_MODE"
}

run_validate() {
  info "running validate for $(echo "$LOCALES_CSV" | tr ',' ' ' | wc -w) locales from $EN_FILE"
  "$TRANSLATOR_BIN" --locale "$LOCALE" \
    validate --langs "$LOCALES_CSV" --en "$EN_FILE"
}

run_status() {
  info "running status for $(echo "$LOCALES_CSV" | tr ',' ' ' | wc -w) locales from $EN_FILE"
  "$TRANSLATOR_BIN" --locale "$LOCALE" \
    status --langs "$LOCALES_CSV" --en "$EN_FILE"
}

if [[ "$MODE" == "-h" || "$MODE" == "--help" ]]; then
  usage
  exit 0
fi

require_files
resolve_translator_bin
load_locales
ensure_locale_files
info "mode=$MODE locale=$LOCALE translator=$TRANSLATOR_BIN"

case "$MODE" in
  all)
    run_translate
    run_validate
    run_status
    ;;
  translate)
    run_translate
    ;;
  validate)
    run_validate
    ;;
  status)
    run_status
    ;;
  *)
    usage
    exit 2
    ;;
esac
