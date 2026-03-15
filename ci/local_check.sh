#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT_DIR"

export CARGO_HOME="${CARGO_HOME:-$ROOT_DIR/.cargo-home}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/.target}"
export CARGO_NET_OFFLINE="${CARGO_NET_OFFLINE:-1}"
mkdir -p "$CARGO_HOME" "$CARGO_TARGET_DIR"

need() {
    command -v "$1" >/dev/null 2>&1
}

hard_need() {
    if ! need "$1"; then
        echo "[error] required tool '$1' is missing" >&2
        exit 1
    fi
}

step() {
    echo
    echo "▶ $*"
}

hard_need cargo
hard_need rustc
hard_need tar

step "Tool versions"
rustc --version
cargo --version

step "cargo metadata"
cargo metadata --format-version 1 --no-deps >/dev/null

if [ -n "$(find crates -mindepth 2 -maxdepth 2 -name Cargo.toml -print -quit)" ]; then
    OFFLINE_ARGS=()
    if [ "${CARGO_NET_OFFLINE}" = "1" ]; then
        OFFLINE_ARGS+=(--offline)
    fi

    step "cargo fmt"
    cargo fmt --all -- --check

    step "cargo clippy"
    cargo clippy --workspace --all-targets "${OFFLINE_ARGS[@]}" -- -D warnings

    step "cargo test"
    cargo test --workspace "${OFFLINE_ARGS[@]}"
else
    step "workspace crates"
    echo "No demo crates found under crates/. Skipping fmt, clippy, and test."
fi

step "package demos"
scripts/package_demos.sh

echo
echo "local_check: all checks passed"
