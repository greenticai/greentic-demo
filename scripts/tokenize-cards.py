#!/usr/bin/env python3
"""Tokenize Adaptive Cards JSON files with ``{{i18n:KEY}}`` markers.

This script rewrites Adaptive Card JSON files in place, replacing literal
strings in known text fields with ``{{i18n:KEY}}`` markers. The greentic
runtime substitutes those tokens at render time
(``greentic-start::resolve_i18n_tokens``), so cards must be tokenized for the
i18n bundles under ``assets/i18n/`` to actually take effect.

The key convention mirrors ``greentic-cards2pack``'s extractor exactly so
that tokens emitted here match the keys produced by
``greentic-cards2pack extract-i18n``. Keep this script in lockstep with
``greentic-cards2pack/src/i18n_extract/extractor.rs`` — if the upstream
extractor changes, this file MUST change too.

Cross-checks: every key emitted is verified against the authoritative
``en.json`` bundle. Any drift (key in cards that is not in en.json, or
vice versa) makes the script exit non-zero so CI / pre-commit catches it.

Exit codes:
    0  success
    2  one or more keys are present in en.json but not produced by the tokenizer
    3  one or more keys produced by the tokenizer are not present in en.json
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

TEXT_FIELDS: set[str] = {
    "text",
    "title",
    "placeholder",
    "label",
    "altText",
    "errorMessage",
    "value",
    "fallbackText",
    "speak",
}

CONTAINER_FIELDS: list[str] = [
    "body",
    "actions",
    "items",
    "columns",
    "inlines",
    "card",
    "inlineAction",
]


def should_extract(text: str, skip_i18n_patterns: bool = True) -> bool:
    """Return True if ``text`` is a literal string that needs translation.

    Mirrors ``should_extract`` in cards2pack's extractor.rs.
    """
    s = text.strip()
    if not s:
        return False
    if skip_i18n_patterns and ("$t(" in s or "$tp(" in s):
        return False
    if s.startswith("{{") and s.endswith("}}"):
        return False
    if s.startswith("${") and s.endswith("}"):
        return False
    return True


def walk(value: Any, prefix: str, path: str, generated: set[str]) -> Any:
    if isinstance(value, dict):
        return walk_object(value, prefix, path, generated)
    if isinstance(value, list):
        out = []
        for i, item in enumerate(value):
            item_path = f"{path}_{i}" if path else str(i)
            out.append(walk(item, prefix, item_path, generated))
        return out
    return value


def walk_object(obj: dict, prefix: str, path: str, generated: set[str]) -> dict:
    out: dict[str, Any] = {}
    for k, v in obj.items():
        if k in TEXT_FIELDS and isinstance(v, str) and should_extract(v):
            key = f"{prefix}.{path}.{k}" if path else f"{prefix}.{k}"
            generated.add(key)
            out[k] = "{{i18n:" + key + "}}"
        elif k in CONTAINER_FIELDS:
            child_path = k if not path else f"{path}_{k}"
            out[k] = walk(v, prefix, child_path, generated)
        elif k == "facts" and isinstance(v, list):
            out[k] = walk_facts(v, prefix, path, generated)
        elif k == "choices" and isinstance(v, list):
            out[k] = walk_choices(v, prefix, path, generated)
        else:
            # Pass through unchanged. Matches cards2pack: it does NOT recurse
            # into arbitrary dict/list values that aren't on the
            # container/facts/choices list.
            out[k] = v
    return out


def walk_facts(facts: list, prefix: str, path: str, generated: set[str]) -> list:
    out = []
    for i, fact in enumerate(facts):
        fact_path = f"{path}_facts_{i}" if path else f"facts_{i}"
        if isinstance(fact, dict):
            new_fact: dict[str, Any] = {}
            for k, v in fact.items():
                if k in ("title", "value") and isinstance(v, str) and should_extract(v):
                    key = f"{prefix}.{fact_path}.{k}"
                    generated.add(key)
                    new_fact[k] = "{{i18n:" + key + "}}"
                else:
                    new_fact[k] = v
            out.append(new_fact)
        else:
            out.append(fact)
    return out


def walk_choices(choices: list, prefix: str, path: str, generated: set[str]) -> list:
    out = []
    for i, choice in enumerate(choices):
        choice_path = f"{path}_choices_{i}" if path else f"choices_{i}"
        if isinstance(choice, dict):
            new_choice: dict[str, Any] = {}
            for k, v in choice.items():
                if k == "title" and isinstance(v, str) and should_extract(v):
                    key = f"{prefix}.{choice_path}.{k}"
                    generated.add(key)
                    new_choice[k] = "{{i18n:" + key + "}}"
                else:
                    new_choice[k] = v
            out.append(new_choice)
        else:
            out.append(choice)
    return out


def tokenize_card_file(
    path: Path,
    en_keys: set[str],
    generated: set[str],
) -> tuple[dict, set[str], set[str]]:
    raw = json.loads(path.read_text(encoding="utf-8"))
    cid = str(raw.get("id") or path.stem)
    prefix = f"card.{cid}"
    pre = set(generated)
    tokenized = walk(raw, prefix, "", generated)
    new_keys = generated - pre
    drifted = new_keys - en_keys
    return tokenized, new_keys, drifted


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(
        description=(
            "Rewrite Adaptive Card JSON files in place with {{i18n:KEY}} markers, "
            "using cards2pack's key convention. Cross-checks against en.json."
        ),
    )
    p.add_argument(
        "--cards-dir",
        type=Path,
        required=True,
        help="Directory containing Adaptive Card *.json files to tokenize in place.",
    )
    p.add_argument(
        "--en-json",
        type=Path,
        required=True,
        help="Path to the authoritative English i18n bundle (assets/i18n/en.json).",
    )
    return p.parse_args()


def main() -> int:
    args = parse_args()

    cards_dir: Path = args.cards_dir
    en_json: Path = args.en_json

    if not cards_dir.is_dir():
        print(f"error: --cards-dir {cards_dir} is not a directory", file=sys.stderr)
        return 1
    if not en_json.is_file():
        print(f"error: --en-json {en_json} is not a file", file=sys.stderr)
        return 1

    en_bundle = json.loads(en_json.read_text(encoding="utf-8"))
    if not isinstance(en_bundle, dict):
        print(f"error: {en_json} must contain a JSON object", file=sys.stderr)
        return 1
    en_keys: set[str] = set(en_bundle.keys())

    card_paths = sorted(cards_dir.glob("*.json"))
    if not card_paths:
        print(f"error: no *.json files found under {cards_dir}", file=sys.stderr)
        return 1

    generated: set[str] = set()
    per_file: list[tuple[Path, dict, int]] = []
    drifted_total: set[str] = set()

    for card_path in card_paths:
        tokenized, new_keys, drifted = tokenize_card_file(card_path, en_keys, generated)
        per_file.append((card_path, tokenized, len(new_keys)))
        if drifted:
            drifted_total.update(drifted)

    if drifted_total:
        print(
            "error: tokenizer produced keys NOT present in en.json (drift):",
            file=sys.stderr,
        )
        for key in sorted(drifted_total):
            print(f"  - {key}", file=sys.stderr)
        return 3

    missing = en_keys - generated
    if missing:
        print(
            "error: en.json contains keys NOT produced by tokenizer (missing):",
            file=sys.stderr,
        )
        for key in sorted(missing):
            print(f"  - {key}", file=sys.stderr)
        return 2

    # All good — write tokenized cards back to disk.
    for card_path, tokenized, count in per_file:
        text = json.dumps(tokenized, indent=2, ensure_ascii=False) + "\n"
        card_path.write_text(text, encoding="utf-8")
        print(f"tokenized {card_path.name}: {count} token(s) added")

    print(f"total keys: {len(generated)} (en.json: {len(en_keys)})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
