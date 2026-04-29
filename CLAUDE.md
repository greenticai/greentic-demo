# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Repo Is

greentic-demo is a Rust workspace that collects independent Greentic demo crates. Each demo crate is a thin wrapper around a pre-built `bundle/` directory containing pack manifests, flows (`.ygtc`), assets, and optionally WASM components. The repo also hosts standalone WASM component sub-crates that compile to `wasm32-wasip2` separately from the workspace.

## Build & Development Commands

```bash
# Build workspace (uses stable toolchain pinned in rust-toolchain.toml)
cargo build --locked

# Format
cargo fmt --all

# Lint (pedantic + all warnings enabled via workspace lints)
cargo clippy --workspace --all-targets -- -D warnings

# Test
cargo test --workspace

# Test a single crate
cargo test -p quickstart-demo

# Local CI mirror (fmt + clippy + test + package demos)
ci/local_check.sh

# Package all demo bundles (requires greentic-bundle tool; skips gracefully if missing)
scripts/package_demos.sh
```

Note: `ci/local_check.sh` runs offline by default (`CARGO_NET_OFFLINE=1`). Set `CARGO_NET_OFFLINE=false` if you need to fetch dependencies.

### Building WASM Components

WASM component sub-crates (excluded from the workspace) require separate builds:

```bash
cd crates/redbutton-demo/component-http   # or component-random, component-betterstack-incident, greentic-http2play
cargo component build --release --target wasm32-wasip2
```

Each component directory has its own `Makefile` with targets: `build`, `wasm`, `check`, `lint`, `test`.

## Architecture

### Workspace Layout

```
crates/<demo-name>/          # Demo crates (workspace members)
  src/lib.rs                 # Exports DEMO_NAME const and bundle_dir()
  bundle/                    # Pre-built bundle (committed to repo)
    bundle.yaml              # References packs, providers, hooks
    packs/<name>.pack/       # Pack with flows, assets, components
      pack.yaml              # Component definitions, flows, assets
      flows/*.ygtc           # Flow definitions
      components/            # WASM component references (if any)
      assets/i18n/           # Locale JSON files
apps/                        # Standalone app packs (not workspace crates)
demos/                       # Output: packaged .gtbundle files
scripts/                     # Packaging scripts
ci/                          # CI scripts
```

### Demo Crate Pattern

Demo crates have zero dependencies — they are metadata wrappers. The `src/lib.rs` is trivial:
```rust
pub const DEMO_NAME: &str = "quickstart-demo";
pub fn bundle_dir() -> &'static str { "bundle" }
```

All meaningful content lives in the `bundle/` directory as YAML manifests, `.ygtc` flows, and pre-compiled WASM.

### WASM Component Sub-Crates

Four component crates under `crates/redbutton-demo/` are excluded from the workspace because they target `wasm32-wasip2`:

- `component-http` — HTTP client operations
- `component-random` — Random value generation
- `component-betterstack-incident` — Better Stack incident integration
- `greentic-http2play` — HTTP to playback bridge

Each is a standalone Rust crate with `crate-type = ["cdylib", "rlib"]`, its own `Cargo.lock`, a `build.rs` for i18n bundling, and a `component.manifest.json` defining operations, capabilities, and schemas. They use `greentic-interfaces-guest` to implement the `greentic:component/component@0.6.0` world.

### Packaging & Publishing

- `scripts/package_demos.sh` archives each crate's `bundle/` into `demos/<name>.gtbundle` (requires `greentic-bundle` CLI)
- CI publishes WASM components to GHCR via ORAS as OCI artifacts
- App packs and demo bundles are also published to GHCR on tagged releases

## Adding a New Demo

1. Create a crate in `crates/<demo-name>/` with a minimal `Cargo.toml` and `src/lib.rs`
2. Add a `bundle/` directory with `bundle.yaml` and at least one pack under `packs/`
3. Run `ci/local_check.sh` to verify

## Localizing a Demo

For demo crates with adaptive cards, see the recipe at `crates/deep-research-demo/assets/i18n/README.md`. Pattern: extract → tokenize cards with `{{i18n:KEY}}` markers → translate via `greentic-i18n-translator` (incremental MT) → ship per-locale bundles in `assets/i18n/`. Runtime substitution by `greentic-start::resolve_i18n_tokens`. For LLM-driven demos, the locale directive lives in `build-answer.json` system prompts using `{{entry.input.metadata.locale}}` runtime template syntax.

## CI Pipeline

GitHub Actions (`ci.yml`) runs `ci/local_check.sh` which executes:
1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `scripts/package_demos.sh`

The publish workflow (`publish.yml`) builds WASM components, publishes packs and bundles to GHCR, and attaches `.gtbundle` files to GitHub Releases on tags.

## Workspace Lints

The workspace enables `clippy::all` and `clippy::pedantic` as warnings. All code must pass these checks.

## Git Conventions

Do NOT add Claude co-author attribution to commits or PRs.
