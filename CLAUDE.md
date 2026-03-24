# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Repo Is

greentic-demo is a thin bootstrap binary that wires environment variables into `greentic-runner-host`. It loads `.env`, discovers tenant packs from the filesystem, builds a `RunnerConfig`, and calls `runner_shim::run(cfg)`. All actual runtime logic (pack resolution, caching, hot reload, telemetry, secrets, routing) lives in the upstream runner crate.

## Build & Development Commands

```bash
# Build (uses nightly for edition 2024)
cargo +nightly build --locked

# Run (sources .env automatically)
make run

# Format
make fmt                    # or: cargo +nightly fmt

# Lint
cargo +nightly clippy --workspace --all-targets --all-features --locked -- -D warnings

# Test (all)
cargo +nightly test --locked

# Test (specific module)
cargo +nightly test --locked loader::tests

# Test with output
cargo +nightly test --locked -- --nocapture

# Local CI mirror (runs fmt + clippy + test + package)
ci/local_check.sh

# Docker
make docker-build DOCKER_IMAGE=greentic-demo:dev
make docker-run

# Cloudflare tunnel for demos
make tunnel
```

Note: The Makefile defaults `CARGO` to `cargo +nightly` because the crate uses Rust edition 2024.

## Architecture

### Startup Flow

```
main.rs → dotenv() → CliArgs::parse() → apply_cli_overrides() → load_packs() → build_runner_config() → runner_shim::run(cfg)
```

### Key Modules

| Module | Purpose |
|--------|---------|
| `cmd/greentic-demo/main.rs` | Binary entry point: env loading, CLI parsing, pack discovery, config building |
| `src/runner_shim/mod.rs` | Feature-gated delegation — re-exports `greentic-runner-host` API or a fallback stub |
| `src/loader.rs` | Scans `PACKS_DIR` for tenant subdirectories with `index.ygtc` + `bindings.yaml` |
| `src/path_safety.rs` | `normalize_under_root()` — prevents directory traversal in pack discovery |

The remaining modules (`config`, `nats_bridge`, `runner_bridge`, `types`, `secrets`, `logging`, `telemetry`, `health`) are gated behind `feature = "runner-shim"` and represent historical NATS bridge utilities kept for reference. New work should go through the runner host.

### Feature Flags

```toml
default = ["use-runner-api"]   # Production: delegates to greentic-runner-host
runner-shim = []               # Fallback stub for testing without the runner
```

Exactly one must be enabled (enforced by `compile_error!`). Use `--no-default-features --features runner-shim` for stub mode.

### Tenant Pack Structure

Each tenant needs a directory under `PACKS_DIR` (default `./packs/`) containing:

```
packs/<tenant-id>/
├── index.ygtc        # Pack manifest (required)
└── bindings.yaml     # Flow adapters, secrets allowlist, MCP config (required)
```

The loader skips directories missing either file (with a warning/error log).

## Configuration

Config flows: `.env` file → env vars → CLI flags (CLI overrides env). See `.env.example` for defaults and `README.md` for the full configuration surface table.

Key variables: `PACKS_DIR`, `PORT`, `PACK_SOURCE`, `PACK_INDEX_URL`, `PACK_CACHE_DIR`, `PACK_REFRESH_INTERVAL`, `TENANT_RESOLVER`, `SECRETS_BACKEND`.

## CI Pipeline

GitHub Actions (`ci.yml`) runs `ci/local_check.sh` which executes:
1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
3. `cargo test --workspace --locked`
4. `cargo package -p greentic-demo --allow-dirty --locked`

## Deployment Demo Pack

`examples/deployment/generic-deploy.gtpack` is included as a reference. If you modify the stub component in `examples/deployment/stub-deploy-component/`, rebuild it:

```bash
cd examples/deployment/stub-deploy-component
cargo build --release --target wasm32-wasip1
# Copy the .wasm to the gtpack components/ directory
```

## Git Conventions

See the workspace-level CLAUDE.md for commit rules. Key point: do NOT add Claude co-author attribution to commits or PRs.
