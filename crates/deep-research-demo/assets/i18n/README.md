# Demo Localization Recipe

This folder holds the i18n bundle for `deep-research-demo`. The pattern
applies to any demo crate with adaptive cards.

## Files

- `en.json` — Canonical English source. Keys follow
  `card.{cardId}.{json_path}.{field}` (the format produced by
  `greentic-cards2pack extract-i18n`).
- `glossary.json` — Terms that must NOT be machine-translated (brand names,
  technical jargon, the `${user_question}` template expression). Edit when MT
  mistranslates a term.
- `{xx}.json` — Per-locale translation, generated from `en.json` via
  `greentic-i18n-translator translate` (codex-cli engine, incremental).
- `.langs` and `.langs.csv` — Gitignored helper files: the target locale list,
  refreshed from `greentic-messaging-providers` webchat-gui at every
  regenerate.

## Runtime substitution

At render time, `greentic-start::resolve_i18n_tokens`
(`greentic-start/src/http_ingress/messaging.rs:382-439`) walks each card body
and substitutes `{{i18n:KEY}}` markers with values from
`assets/i18n/{locale}.json`, falling back to `en.json`, falling back to the
raw key string. Locale comes from `envelope.metadata["locale"]`, set by the
webchat-gui provider from the user's `?lang=` URL parameter.

**Cards must contain `{{i18n:KEY}}` tokens for substitution to fire.**
`cards2pack extract-i18n` only emits the bundle; it does NOT rewrite cards.
The repo's `scripts/tokenize-cards.py` adds the markers — see Regenerate
below.

## LLM system prompts

The two LLM steps in `build-answer.json` (`research_planner`,
`research_analyst`) include a directive
`IMPORTANT: Respond in the locale "{{entry.input.metadata.locale}}".`
The runtime template engine in `greentic-runner-host` resolves
`{{entry.input.metadata.locale}}` from envelope metadata before the prompt
reaches the model.

## Regenerate after card edits

```bash
crates/deep-research-demo/scripts/regenerate-i18n.sh
```

Idempotent. Pulls the latest target locale list from
`greentic-messaging-providers/packs/messaging-webchat-gui/assets/webchat-gui/i18n/`,
re-extracts `en.json`, re-tokenizes cards, re-translates only changed keys,
validates coverage.

Required tools: `greentic-cards2pack`, `greentic-i18n-translator`, `uv`, `jq`.

## Adding a new locale

Add the `.json` file to webchat-gui's locale folder first (so the
locale-picker can offer it), then re-run `regenerate-i18n.sh` here. The
script auto-mirrors webchat-gui's list.

## Quality gates

`cargo test -p deep-research-demo` runs two test suites that catch:

- Locale count mismatch (expected 65)
- Key drift across locales
- Empty translations
- Placeholder / `${var}` template / newline drift
- Char-class violations (e.g. `zh.json` must contain CJK)
- Length-ratio anomalies (translated vs en outside 0.3×–4.0×)
- English bleed-through in non-Latin-script locales
- Tokens that resolve to raw key string at render time

## Quality caveats

Pure machine translation, no human review. Languages with weak codex-cli
coverage:
`qu` (Quechua), `nah` (Nahuatl), `ay` (Aymara), `gn` (Guaraní),
`ht` (Haitian Creole), `lo` (Lao), `si` (Sinhala), `km` (Khmer).
For external-facing demos in any of these, schedule a human review pass.

## Reusing this recipe in another demo crate

1. Copy `glossary.json` and edit terms for the demo's domain.
2. Copy `scripts/regenerate-i18n.sh`, adjust `CRATE_DIR` if structure differs.
3. Run the script. Verify gtpack inclusion with
   `unzip -l demos/<demo>.gtpack | grep i18n`.
4. Add the same two test files (`tests/i18n_bundle.rs`,
   `tests/i18n_render.rs`), updating `EXPECTED_LOCALE_COUNT` and sample
   locales as needed.
