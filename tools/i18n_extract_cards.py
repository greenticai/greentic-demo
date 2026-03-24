#!/usr/bin/env python3
"""Extract translatable strings from Adaptive Card JSONs, rewrite cards with $t() syntax,
and generate i18n translation bundles.

Usage:
    python3 tools/i18n_extract_cards.py apps/quickstart-app
    python3 tools/i18n_extract_cards.py apps/quickstart-app --dry-run
    python3 tools/i18n_extract_cards.py --all --apps-dir apps
"""

import argparse
import json
import re
import sys
from pathlib import Path

TRANSLATABLE_FIELDS = {"text", "title", "placeholder", "label", "altText", "errorMessage", "fallbackText", "speak"}
CONTAINER_FIELDS = {"body", "actions", "items", "columns", "inlines", "card", "inlineAction"}
TEMPLATE_ONLY_RE = re.compile(r"^\{\{[^}]+\}\}$")
I18N_MARKER_RE = re.compile(r"^\$t\(|^\$tp\(")

SUPPORTED_LOCALES = [
    "ar", "ar-AE", "ar-DZ", "ar-EG", "ar-IQ", "ar-MA", "ar-SA", "ar-SD", "ar-SY", "ar-TN",
    "ay", "bg", "bn", "cs", "da", "de", "el", "en", "en-GB", "es", "et", "fa", "fi", "fr",
    "gn", "gu", "hi", "hr", "ht", "hu", "id", "it", "ja", "km", "kn", "ko", "lo", "lt", "lv",
    "ml", "mr", "ms", "my", "nah", "ne", "nl", "no", "pa", "pl", "pt", "qu", "ro", "ru",
    "si", "sk", "sr", "sv", "ta", "te", "th", "tl", "tr", "uk", "ur", "vi", "zh",
]


def card_id_from_file(filepath: Path) -> str:
    """Derive card_id from filename (e.g. welcome_card.json -> welcome)."""
    stem = filepath.stem
    if stem.endswith("_card"):
        stem = stem[: -len("_card")]
    return stem


def should_translate(value: str) -> bool:
    """Check if a string value should be extracted for translation."""
    if not value or not value.strip():
        return False
    if TEMPLATE_ONLY_RE.match(value.strip()):
        return False
    if I18N_MARKER_RE.match(value.strip()):
        return False
    return True


def extract_and_rewrite(node, path_parts: list, pack_prefix: str, card_id: str, keys: dict):
    """Recursively extract translatable strings and rewrite them with $t() markers."""
    modified = False

    if isinstance(node, dict):
        for field in TRANSLATABLE_FIELDS:
            if field in node and isinstance(node[field], str):
                value = node[field]
                if should_translate(value):
                    key = build_key(pack_prefix, card_id, path_parts, field)
                    keys[key] = value
                    node[field] = f"$t({key})"
                    modified = True

        if "facts" in node and isinstance(node["facts"], list):
            for i, fact in enumerate(node["facts"]):
                if isinstance(fact, dict):
                    for fact_field in ("title", "value"):
                        if fact_field in fact and isinstance(fact[fact_field], str):
                            value = fact[fact_field]
                            if should_translate(value):
                                key = build_key(pack_prefix, card_id, path_parts, f"facts_{i}_{fact_field}")
                                keys[key] = value
                                fact[fact_field] = f"$t({key})"
                                modified = True

        if "choices" in node and isinstance(node["choices"], list):
            for i, choice in enumerate(node["choices"]):
                if isinstance(choice, dict) and "title" in choice and isinstance(choice["title"], str):
                    value = choice["title"]
                    if should_translate(value):
                        key = build_key(pack_prefix, card_id, path_parts, f"choices_{i}_title")
                        keys[key] = value
                        choice["title"] = f"$t({key})"
                        modified = True

        for container_field in CONTAINER_FIELDS:
            if container_field in node:
                child = node[container_field]
                if isinstance(child, list):
                    for i, item in enumerate(child):
                        child_path = path_parts + [f"{container_field}_{i}"]
                        if extract_and_rewrite(item, child_path, pack_prefix, card_id, keys):
                            modified = True
                elif isinstance(child, dict):
                    child_path = path_parts + [container_field]
                    if extract_and_rewrite(child, child_path, pack_prefix, card_id, keys):
                        modified = True

    elif isinstance(node, list):
        for i, item in enumerate(node):
            child_path = path_parts + [str(i)]
            if extract_and_rewrite(item, child_path, pack_prefix, card_id, keys):
                modified = True

    return modified


def build_key(pack_prefix: str, card_id: str, path_parts: list, field: str) -> str:
    """Build a flat i18n key from path components."""
    path_str = ".".join(path_parts) if path_parts else ""
    if path_str:
        return f"{pack_prefix}.{card_id}.{path_str}.{field}"
    return f"{pack_prefix}.{card_id}.{field}"


def find_cards_dir(pack_dir: Path) -> Path | None:
    """Find the cards directory within a pack."""
    candidates = [
        pack_dir / "assets" / "cards",
        pack_dir / "assets",
        pack_dir / "cards",
    ]
    for candidate in candidates:
        if candidate.exists() and any(candidate.glob("*.json")):
            return candidate
    return None


def process_pack(pack_dir: Path, dry_run: bool = False) -> dict:
    """Process all adaptive card JSONs in a pack directory."""
    cards_dir = find_cards_dir(pack_dir)
    if not cards_dir:
        print(f"  No card JSON files found in {pack_dir}")
        return {}

    card_files = sorted(cards_dir.glob("*.json"))
    pack_name = pack_dir.name
    pack_prefix = pack_name.replace("-", "_")
    all_keys = {}

    for card_file in card_files:
        try:
            card_data = json.loads(card_file.read_text())
        except (json.JSONDecodeError, OSError) as exc:
            print(f"  Skipping {card_file.name}: {exc}")
            continue

        if card_data.get("type") != "AdaptiveCard":
            continue

        card_id = card_data.get("greentic", {}).get("cardId") or card_id_from_file(card_file)
        card_keys = {}

        extract_and_rewrite(card_data, [], pack_prefix, card_id, card_keys)

        if card_keys:
            all_keys.update(card_keys)
            if not dry_run:
                card_file.write_text(json.dumps(card_data, indent=2, ensure_ascii=False) + "\n")
                print(f"  Rewrote {card_file.name} ({len(card_keys)} keys)")
            else:
                print(f"  [dry-run] {card_file.name}: {len(card_keys)} keys")

    if all_keys and not dry_run:
        i18n_dir = pack_dir / "assets" / "i18n"
        i18n_dir.mkdir(parents=True, exist_ok=True)

        en_bundle = dict(sorted(all_keys.items()))
        en_path = i18n_dir / "en.json"
        en_path.write_text(json.dumps(en_bundle, indent=2, ensure_ascii=False) + "\n")
        print(f"  Wrote {en_path} ({len(en_bundle)} keys)")

        locales_written = ["en"]
        for locale in SUPPORTED_LOCALES:
            if locale == "en":
                continue
            locale_path = i18n_dir / f"{locale}.json"
            if not locale_path.exists():
                locale_path.write_text(json.dumps(en_bundle, indent=2, ensure_ascii=False) + "\n")
                locales_written.append(locale)

        locales_meta = i18n_dir / "locales.json"
        locales_meta.write_text(json.dumps(sorted(locales_written), indent=2) + "\n")
        print(f"  Created {len(locales_written)} locale bundles")

        update_pack_yaml_assets(pack_dir, locales_written)

    return all_keys


def update_pack_yaml_assets(pack_dir: Path, locales: list):
    """Add i18n assets to pack.yaml if not already present."""
    pack_yaml = pack_dir / "pack.yaml"
    if not pack_yaml.exists():
        return

    content = pack_yaml.read_text()

    i18n_entries = []
    for locale in sorted(locales):
        rel_path = f"assets/i18n/{locale}.json"
        if rel_path not in content:
            i18n_entries.append(f"- path: {rel_path}")

    locales_rel = "assets/i18n/locales.json"
    if locales_rel not in content:
        i18n_entries.append(f"- path: {locales_rel}")

    if not i18n_entries:
        return

    if "assets: []" in content:
        insert_text = "\n".join(i18n_entries)
        content = content.replace("assets: []", f"assets:\n{insert_text}")
    elif "assets:" in content:
        insert_text = "\n".join(i18n_entries)
        content = content.replace("assets:", f"assets:\n{insert_text}", 1)
    else:
        insert_text = "assets:\n" + "\n".join(i18n_entries)
        content = content.rstrip() + "\n" + insert_text + "\n"

    pack_yaml.write_text(content)
    print(f"  Updated pack.yaml with {len(i18n_entries)} i18n asset entries")


def main():
    parser = argparse.ArgumentParser(description="Extract i18n from Adaptive Cards and generate translation bundles")
    parser.add_argument("pack_dir", nargs="?", help="Path to pack directory")
    parser.add_argument("--all", action="store_true", help="Process all packs")
    parser.add_argument("--apps-dir", default="apps", help="Root apps directory (default: apps)")
    parser.add_argument("--packs-dir", default="packs", help="Root packs directory (default: packs)")
    parser.add_argument("--dry-run", action="store_true", help="Preview without modifying files")
    args = parser.parse_args()

    if not args.pack_dir and not args.all:
        parser.print_help()
        sys.exit(1)

    if args.all:
        total_keys = 0
        for search_dir in [args.apps_dir, args.packs_dir]:
            root = Path(search_dir)
            if not root.exists():
                continue
            for pack_dir in sorted(root.iterdir()):
                if not pack_dir.is_dir():
                    continue
                print(f"\nProcessing {pack_dir}...")
                keys = process_pack(pack_dir, dry_run=args.dry_run)
                total_keys += len(keys)

        print(f"\nTotal: {total_keys} translatable keys")
    else:
        pack_path = Path(args.pack_dir)
        if not pack_path.exists():
            print(f"Pack directory not found: {pack_path}")
            sys.exit(1)

        print(f"Processing {pack_path.name}...")
        keys = process_pack(pack_path, dry_run=args.dry_run)
        print(f"\nTotal: {len(keys)} translatable keys")


if __name__ == "__main__":
    main()
