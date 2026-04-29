---
title: Deep Research Demo — Multi-Language (24h quality bar; skin work split out)
date: 2026-04-29
authors: Bima Pangestu, Claude
status: approved 2026-04-29; re-scoped 2026-04-29 to translation-only; revised 2026-04-29 (Phase A) after runtime/build investigation closed all "discover X" placeholders
related: 3Point P0 demo escalation; TrainingBranding.zip (delivered 2026-04-29)
target_execution_date: 2026-04-30
quality_bar: no technical debt — every shortcut from prior plan revisions was investigated and either resolved or explicitly transferred to a documented follow-up
---

# Goal

Tomorrow (2026-04-30), within a 24h budget, ship a **production-quality** localization of `greentic-demo/crates/deep-research-demo` into webchat-gui's existing 65 non-English locales:

- Adaptive cards render in the user-selected language via runtime `{{i18n:KEY}}` token substitution
- LLM `system` prompts deterministically instruct the model to respond in the user's locale via runtime template interpolation of `{{entry.input.metadata.locale}}`
- Automated tests guard the i18n bundle against drift, English bleed-through, and per-locale char-class violations on future PRs
- A checked-in `regenerate-i18n.sh` script makes re-translation idempotent for any future card edit

Lens: **quality > features**. No "TBD" / "discover X at runtime" / throwaway scripts / orphan docs. Anything we cannot do correctly in 24h is explicitly transferred to a follow-up section, not silently shipped.

# Scope

## In

- Place locale bundles at `crates/deep-research-demo/assets/i18n/{xx}.json` — verified path: 6 reference demos (quickstart, hr-onboarding, helpdesk-itsm, sales-crm, supply-chain, greentic-ai) already use this structure. `package_demos.sh:519-522` recursively copies `assets/`, so dropping `assets/i18n/` is auto-included in the gtpack.
- Translate adaptive cards in `crates/deep-research-demo/assets/cards/{main_menu,research_plan,final_report}.json` into webchat-gui's 65 non-English locales using `greentic-cards2pack extract-i18n` + `greentic-i18n-translator translate`. Cards must end up containing `{{i18n:card.{cardId}.{path}.{field}}}` tokens; `assets/i18n/{xx}.json` provides the per-locale substitutions.
- Append a deterministic locale directive to BOTH `research_planner` and `research_analyst` LLM `system` content strings inside `crates/deep-research-demo/build-answer.json`, using already-proven runtime template syntax: `\n\nIMPORTANT: Respond in the locale "{{entry.input.metadata.locale}}". If unknown, fall back to English.`
- Add 2 automated tests in `crates/deep-research-demo/tests/`:
  - `i18n_bundle.rs` — bundle structure validation (extends what `greentic-i18n-translator::validate_lang_map` covers, plus per-locale char-class checks, length-ratio sanity, English-bleed-through detection)
  - `i18n_render.rs` — card-render smoke (load each card, walk i18n keys, assert every key resolves to a non-empty non-key-fallback value across a sample of locales)
- Add `crates/deep-research-demo/scripts/regenerate-i18n.sh` (idempotent: re-extract → re-translate incremental → re-validate → echo summary)
- Add `crates/deep-research-demo/assets/i18n/README.md` documenting the recipe; link from `greentic-demo/CLAUDE.md` and from the demo crate's `README.md`
- Mirror this spec into the greentic-demo repo at `greentic-demo/docs/superpowers/specs/2026-04-29-deep-research-demo-i18n-design.md` so the audit trail is git-versioned alongside the code change

## Out

- Translating the English BODY of `planner_system_prompt` / `researcher_system_prompt` itself — kept English (LLMs reason better in English; locale steered via the directive)
- Touching `demos/deep-research-demo-setup-answers.json` — setup engine has zero interpolation (verified: `greentic-setup/src/setup_to_formspec/convert.rs:81-89` writes verbatim, `greentic-setup/src/engine/executors.rs:174-181` persists verbatim). Directive cannot live there.
- Touching `crates/deep-research-demo/assets/setup.yaml` — operator-facing config, end-user never sees it
- Webchat-gui changes — already 66-locale, already plumbs `?lang=` → DirectLine → `envelope.metadata.locale` (verified via `greentic-start/src/http_ingress/messaging.rs:57-62`)
- Other demos (helpdesk-itsm, hr-onboarding) — same recipe reusable, not in scope today
- Skin work — separate project (see Follow-up section at bottom)
- Human review of MT output — pure machine translation; per-locale risks documented in PR. Auto-detection of MT defects via the bundle-structure test (char class, length, English bleed) is the substitute.
- Adding multi-lang nightly to greentic-e2e — separate gap (per memory: e2e coverage is broadly thin); deferred to follow-up

# Approach

## Translation pipeline (cards2pack + i18n-translator)

- `greentic-cards2pack extract-i18n --input assets/cards --output assets/i18n/en.json` → produces canonical English bundle keyed `card.{cardId}.{json_path}.{field}`. **First implementation step diffs cards before/after extract** to determine whether cards2pack tokenizes cards in-place. If yes (expected): adopt as-is. If no: write a small in-repo Python tokenizer (~50 lines, uv-managed) that walks card JSON paths, replaces each text field with `{{i18n:card.{cardId}.{path}.{field}}}`, using the en.json key set as authority.
- Author `assets/i18n/glossary.json` (do-not-translate terms: `Greentic`, `Deep Research`, `Digital Worker`, `Planner`, `Researcher`, `Agentic`, `Single Shot`, `OpenAI`, `Ollama`, `LLM`, `Adaptive Card`).
- Mirror webchat-gui's 65 non-English locales: `greentic-i18n-translator translate --langs <list> --en assets/i18n/en.json --glossary assets/i18n/glossary.json`. Pipeline is incremental hash-based via `.i18n/translator-state.json` — first run is full spend.
- Validate: `greentic-i18n-translator validate --langs <list> --en assets/i18n/en.json` covers missing keys, empty values, `{}` placeholder count, newline count, backtick-span content (verified via `greentic-i18n-translator/src/validate.rs:110-129`).

## LLM locale directive (build-answer.json)

Edit `crates/deep-research-demo/build-answer.json`:

- Append to `system` content of `research_planner` step (around line 186):
  `\n\nIMPORTANT: Respond in the locale "{{entry.input.metadata.locale}}". If unknown, fall back to English.`
- Append the same line to `system` content of `research_analyst` step (around line 266).

Runtime path is already validated: `greentic-runner-host/src/runner/engine.rs::template_context()` (lines 1659-1672) exposes `entry`, `in`, `prev`, `node`, `state`. The flow `entry` is built by `greentic-start/src/messaging_app.rs::run_app_flow()` (line 214) as `json!({ "input": envelope, ... })`. Webchat-gui provider stamps `envelope.metadata["locale"]` (per `messaging-provider-webchat/src/ops/ingest.rs` line 245). Therefore `{{entry.input.metadata.locale}}` resolves at runtime — proven by the same file's existing usage of `{{entry.input.metadata.user_question}}`, `provider`, `model`, `url` (lines 190, 270).

**Why this is the right seam (not the alternatives investigated and rejected):**
- **Setup-answers (rejected)**: setup engine writes verbatim, no template engine.
- **Setup.yaml defaults (rejected)**: same — verbatim, plus operator-time only, no per-request rendering.
- **Flow `template` node injecting state.locale (rejected as overkill)**: would require new flow nodes; runtime engine already exposes `entry.input.metadata.locale` directly to LLM step config strings — no template node hop needed.

## Pack rebuild

`bash greentic-demo/scripts/package_demos.sh deep-research-demo` — verified entry point. The script's `cp -R "$source_assets_dir/." "$temp_pack_dir/assets/"` (line 521) auto-includes the new `assets/i18n/` subtree. Output goes to `demos/deep-research-demo.gtpack` (line 583). Verify with `unzip -l demos/deep-research-demo.gtpack | grep -c 'assets/i18n/.*\.json$'` — expect 66.

## Automated tests

- `crates/deep-research-demo/tests/i18n_bundle.rs` — Pure-Rust unit test, no runtime boot. For each `assets/i18n/*.json`:
  - Reuse `greentic_i18n_translator::validate::{validate_lang_map, JsonMap}` to assert: same key set as `en.json`, no empty values, placeholder counts match, newline counts match, backtick spans preserved
  - **Custom: char-class regex per locale** — `ru/uk/sr/bg/mk` must contain Cyrillic, `zh/ja/ko` must contain CJK, `ar*/fa/ur` must contain Arabic, `hi/mr/ne/sa` must contain Devanagari, `th/lo/km/my` must contain native scripts, `ta/te/kn/ml` must contain respective Indic scripts, `el` must contain Greek
  - **Custom: length-ratio sanity** — translated string length within 0.4×–3.0× of EN (catches truncated MT or hallucination runaway)
  - **Custom: English-bleed-through detection** — for non-Latin-script locales, fail if any value contains 3+ consecutive English-only ASCII words (matched against a small stop-word list: `the,and,for,with,from,this,that,...`)
- `crates/deep-research-demo/tests/i18n_render.rs` — Integration smoke. For sample of locales (en, fr, ja, ar, hi, id):
  - Load each `assets/cards/*.json`
  - Use `greentic_cards2pack::i18n_extract` to compute the key set the card needs
  - Verify each key resolves to a non-empty value in the locale bundle (NOT to the raw key string — that would mean missing-key fallback at runtime)

## Regenerate script

`crates/deep-research-demo/scripts/regenerate-i18n.sh` — idempotent end-to-end:
1. Re-run `cards2pack extract-i18n`
2. Re-run `i18n-translator translate` (incremental — only changed keys re-translate)
3. Re-run `i18n-translator validate`
4. Echo summary: locale count, key count, drift report, suspected-quality-issue count from custom checks

Documented in `assets/i18n/README.md` and linked from greentic-demo top-level `CLAUDE.md` (or `docs/`).

## Spec mirror

Copy this spec into the greentic-demo repo at `docs/superpowers/specs/2026-04-29-deep-research-demo-i18n-design.md` so the design doc is git-versioned with the code change. Cross-link from greentic-demo's `CLAUDE.md` (or top-level `docs/index.md` if it exists).

# Data flow

```
User selects language in webchat-gui locale-picker (?lang=fr)
  → DirectLine carries locale to messaging-provider-webchat
  → Provider stamps envelope.metadata.locale = "fr"
      (greentic-messaging-providers/.../webchat/src/ops/ingest.rs:245)
  → greentic-start::messaging_app::run_app_flow() puts envelope into entry.input
      (greentic-start/src/messaging_app.rs:214)

  Card render path:
    → greentic-start::resolve_i18n_tokens() walks card body
        (greentic-start/src/http_ingress/messaging.rs:382-439)
    → reads assets/i18n/fr.json from gtpack zip via read_i18n_bundle()
    → substitutes {{i18n:card.menu.body_0.text}} → bundle["card.menu.body_0.text"]
    → fallback chain: fr.json → en.json → raw key string

  LLM step path (research_planner / research_analyst):
    → runtime template engine resolves entry.input.metadata.locale → "fr"
        (greentic-runner-host/src/runner/engine.rs::template_context, line 1659-1672)
    → LLM receives system: "...IMPORTANT: Respond in the locale \"fr\"..."
    → LLM output in French (deterministic via directive, not context inference)
```

# Risks & open questions

- **cards2pack tokenization behavior** — Whether `extract-i18n` modifies cards in-place to add `{{i18n:KEY}}` tokens is empirically untested by us. **Mitigation:** First implementation step diffs cards before/after extract; if not tokenized, fall back to the in-repo Python tokenizer described in Approach. Plan must NOT assume; both paths are documented.
- **BCP-47 code understanding by LLM** — Modern LLMs handle common codes (`fr`, `id`, `ja`, `zh`) well; obscure codes (`nah`/Nahuatl, `qu`/Quechua, `gn`/Guaraní, `ay`/Aymara, `nah`) may confuse. **Mitigation:** directive includes "If unknown, fall back to English." For demo runs in those locales, recommend a brief manual smoke test.
- **Exotic-language MT quality** — MT for `qu`, `nah`, `ay`, `gn`, `ht`, `lo`, `si`, `km` may be poor. **Mitigation:** char-class test (Task 9) catches blank/wrong-script output; length-ratio test catches truncation; English-bleed-through test catches partial translation. Failures fail CI rather than ship silently. PR description flags exotics as "MT, not human-reviewed."
- **Missing-key runtime fallback** — Runtime renders raw key string if a key is in en.json but missing in the locale bundle (verified: `greentic-start/src/http_ingress/messaging.rs:422`, no per-key EN fallback). **Mitigation:** bundle-structure test (Task 9) fails on key drift, so any drift fails CI before reaching runtime.
- **LLM bleed in early conversation** — On the very first message before user types, LLM may default to English even with directive. **Acceptable:** cards already render in user's language (immediate localized UX). LLM kicks in on user's first question (which is in their language, providing clear context for the LLM).
- **Single-repo PR** — Confined to `greentic-demo`. Per memory feedback: pull latest before branching; feature from `develop` if remote has it, else from `main`.

# Verification

- `bash ci/local_check.sh` in `greentic-demo` (fmt + clippy + test + repo extras). Required green before PR. Now meaningful because tests in Task 9-10 are part of the test target.
- **`cargo test -p deep-research-demo` (or workspace test)** — bundle-structure + render-smoke tests must pass.
- **`greentic-i18n-translator validate`** as a separate gate from cargo tests — must report no missing/empty/placeholder issues.
- **Demo smoke test**: `bash scripts/package_demos.sh deep-research-demo`, then `gtc setup` + `gtc start`, switch webchat-gui locale to `fr` and `id`, confirm cards in target language AND LLM response in target language. No console errors about missing i18n keys.
- **Spot-check 5 high-value locales** (`fr`, `de`, `ja`, `zh`, `id`): glossary terms preserved (no translated `Greentic` etc.), key set matches en.json, length sanity holds.

# Rollback

Revert PR. Old literal-English cards still ship; `build-answer.json` reverts to English-only system prompts. No database / no infra rollback. The new `assets/i18n/` folder + tests cleanly disappear.

# Out-of-scope follow-ups

- Apply same recipe to `helpdesk-itsm-demo-bundle`, `hr-onboarding-demo-bundle` — pattern is mechanical with the README + regenerate script in place.
- Lift the `assets/i18n/` convention into a documented section of `greentic-demo/CLAUDE.md` ("how to localize a demo crate"), with reference to this spec.
- Replace BCP-47 code in directive with human-readable language name (`French` not `fr`) via a small lookup table — drops the LLM-understanding risk but adds maintenance.
- Add `greentic-e2e` nightly that flips locale and asserts demo behavior — currently zero multi-lang e2e coverage in the repo.
- Tenant-generic skin authoring via `greentic-bundle wizard` — see Follow-up project below.

# Follow-up project (split out from this spec, not part of tomorrow)

**Tenant-generic skin authoring via `greentic-bundle wizard`.**

The original spec bundled "drop a `skins/3point-training/` folder into webchat-gui" alongside the translation work. After review, this was rejected as a one-off solution incompatible with the long-term lens (Bima: *"harus generic solution, next-nya bisa tenant lain selain 3Point"*). The right architecture is a new wizard step in `greentic-bundle` — likely implemented as a `greentic-bundle-extensions` reference extension implementing `greentic:extension-bundle@0.1.0` — that scaffolds a per-tenant skin from `_template/` into the bundle workspace (not into the webchat-gui pack itself, keeping the pack tenant-agnostic).

Estimated effort: **3-5 days**. Touches `greentic-bundle` (wizard step + JSON schema + interactive/validate/apply modes), `greentic-bundle-extensions` (new reference extension, WIT contract, template engine for `skin.json` + `styleOptions.json`), and `greentic-messaging-providers` (eat-our-own-dogfood: generate `3point-training` via the wizard once it exists).

This needs its own brainstorming pass before any implementation — too cross-cutting to spec inline here. Trigger that brainstorm when ready (likely after tomorrow's translation PR ships).

The `TrainingBranding.zip` artifacts (`shell.css` + 9 screenshots) should be retained somewhere accessible (e.g., committed to greentic-bundle's testdata or held in a shared drive path referenced from the future spec) so the wizard project can reference them as the V1-parity target.

`shell.css` notes for the future spec author:
- 1071 lines, ported from V1 Greentic skin (TASK-001..004)
- Dark + light themes via `:root` + `[data-theme="light"]` blocks
- Brand colors: `#06b6d4` (cyan brand), Inter / Orbitron / JetBrains Mono fonts
- Includes `.fullpage-shell`, `.topbar`, and many other selectors targeting the V1 webchat-gui DOM — selector compatibility against current webchat-gui DOM needs verification
- Brand assets (logo.svg, hero.jpg, favicon.ico) NOT in zip — request from 3Point separately
