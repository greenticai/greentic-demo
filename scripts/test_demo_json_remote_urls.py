#!/usr/bin/env python3

from pathlib import Path
import json
import re


FORBIDDEN_PATTERNS = [
    re.compile(r"file://"),
    re.compile(r"(?:^|\s)\./"),
    re.compile(r"(?:^|\s)\.\./"),
    re.compile(r"(?:^|\s)demos/[^\" ]+"),
    re.compile(r"(?:^|\s)(?:\./|\.\./)?[^:/?#\s]+\.gtpack(?:$|[?#\s])"),
]


def walk(value, path="root"):
    if isinstance(value, dict):
        for key, child in value.items():
            walk(child, f"{path}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            walk(child, f"{path}[{index}]")
    elif isinstance(value, str):
        for pattern in FORBIDDEN_PATTERNS:
            assert not pattern.search(value), (
                f"Local file reference found at {path}: {value}"
            )


def test_demo_json_uses_remote_urls():
    for file in Path("demos").rglob("*.json"):
        data = json.loads(file.read_text())
        walk(data, str(file))


if __name__ == "__main__":
    test_demo_json_uses_remote_urls()
