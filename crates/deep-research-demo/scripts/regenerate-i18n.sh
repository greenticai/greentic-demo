#!/usr/bin/env bash
# Regenerate the deep-research-demo i18n bundle end-to-end. Idempotent.
#
# Steps:
#   1. Refresh target locale list from webchat-gui's i18n (single source of truth)
#   2. Re-extract English bundle from cards via greentic-cards2pack
#   3. Re-tokenize cards with {{i18n:KEY}} markers (cards2pack extract-i18n
#      doesn't tokenize cards; in-repo Python tokenizer does that)
#   4. Re-translate via greentic-i18n-translator (incremental — only changed keys)
#   5. Validate coverage
#   6. Print summary
#
# Run after editing assets/cards/*.json or after webchat-gui adds/drops a locale.

set -euo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKSPACE_ROOT="$(cd "$CRATE_DIR/../../.." && pwd)"
WEBCHAT_I18N="$WORKSPACE_ROOT/greentic-messaging-providers/packs/messaging-webchat-gui/assets/webchat-gui/i18n"
I18N_DIR="$CRATE_DIR/assets/i18n"
DEMO_REPO_ROOT="$(cd "$CRATE_DIR/../.." && pwd)"
TOKENIZER="$DEMO_REPO_ROOT/scripts/tokenize-cards.py"

cd "$CRATE_DIR"

echo "==> Refreshing target locale list from webchat-gui"
ls "$WEBCHAT_I18N" \
  | grep -E '^[a-z]+(-[A-Z]+)?\.json$' \
  | sed 's/\.json$//' \
  | grep -v '^en$' \
  | sort \
  > "$I18N_DIR/.langs"
paste -sd, "$I18N_DIR/.langs" > "$I18N_DIR/.langs.csv"
LOCALE_COUNT=$(wc -l < "$I18N_DIR/.langs")
echo "    $LOCALE_COUNT locales targeted"

echo "==> Re-extracting English bundle from cards"
greentic-cards2pack extract-i18n \
  --input assets/cards \
  --output "$I18N_DIR/en.json"
KEY_COUNT=$(jq 'length' "$I18N_DIR/en.json")
echo "    $KEY_COUNT keys extracted"

echo "==> Re-tokenizing cards with {{i18n:KEY}} markers"
uv run python "$TOKENIZER" \
  --cards-dir assets/cards \
  --en-json "$I18N_DIR/en.json"

echo "==> Re-translating (incremental — only changed keys)"
LANGS=$(cat "$I18N_DIR/.langs.csv")
greentic-i18n-translator translate \
  --langs "$LANGS" \
  --en "$I18N_DIR/en.json" \
  --glossary "$I18N_DIR/glossary.json"

echo "==> Validating coverage"
greentic-i18n-translator validate \
  --langs "$LANGS" \
  --en "$I18N_DIR/en.json"

echo "==> Done."
echo "    locales: $LOCALE_COUNT"
echo "    keys:    $KEY_COUNT"
echo "    bundle:  $I18N_DIR"
echo
echo "Next: 'cargo test -p deep-research-demo' to run bundle-structure + render tests."
