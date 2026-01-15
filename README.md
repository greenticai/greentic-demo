# greentic-demo

Thin bootstrap that wires environment variables into `greentic-runner-host`. All runtime logic (pack resolution, caching, hot reload, telemetry, secrets, routing) happens inside the runner; this crate simply loads `.env`, builds a stable `RunnerConfig`, and calls `runner_shim::run(cfg)` so future runner releases can drop in without churn.

## Quickstart

1. Copy the defaults and edit as needed:
   ```bash
   cp .env.example .env
   $EDITOR .env
   ```
   Make sure `PACKS_DIR` points to a directory where each tenant has a `bindings.yaml` file.
2. Start the runner with your packs/index:
   ```bash
   make run
   ```
   `make run` sources `.env`, then runs `cargo +nightly run --locked --bin greentic-demo` so you always test the same dependency graph as CI.
3. Update `.env` whenever you switch pack backends, cache directories, refresh intervals, or tenant routing strategies. `dotenvy` loads the file automatically at startup.

## Docker Image

The multi-stage `Dockerfile` builds a MUSL binary and copies it into `gcr.io/distroless/static:nonroot`, keeping the final image around 25–30 MB. Targets cover the common flow:

```bash
make docker-build DOCKER_IMAGE=ghcr.io/greentic-ai/greentic-demo:local
make docker-run   DOCKER_IMAGE=ghcr.io/greentic-ai/greentic-demo:local
```

`docker-run` reads the current `.env` file and publishes `${PORT:-8080}` by default.

## Cloudflared Tunnel

Expose a local instance through Cloudflare Tunnel without poking firewall holes:

```bash
make tunnel
```

This target checks for `cloudflared`, sources `.env`, and runs `cloudflared tunnel --url http://127.0.0.1:$PORT`. See Cloudflare's docs for installing the CLI and authenticating your account.

## Configuration Surface

`cmd/greentic-demo/main.rs` converts env vars into a stable `RunnerConfig`. You can also pass CLI flags to override the environment. The key knobs are:

| Variable | Description | Default |
| --- | --- | --- |
| `PACKS_DIR` | Directory containing per-tenant `bindings.yaml` files | `./packs` |
| `PORT` | HTTP listener exposed by the runner host | `8080` |
| `SECRETS_BACKEND` | Hint for which secrets backend to bootstrap (`env`, `aws`, `gcp`, `azure`) | `env` |
| `PACK_SOURCE` | Resolver scheme (`fs`, `http`, `oci`, `s3`, `gcs`, `azblob`) | `fs` |
| `PACK_INDEX_URL` | Local path or URL to `index.json` | `./examples/index.json` |
| `PACK_CACHE_DIR` | Content-addressed cache root | `.packs` |
| `PACK_REFRESH_INTERVAL` | Human-friendly interval (e.g. `30s`, `5m`) for hot-reload polling | `30s` |
| `TENANT_RESOLVER` | Routing strategy: `host`, `header`, `jwt`, or `env` | `host` |
| `PACK_PUBLIC_KEY` | Optional Ed25519 key to verify signed packs | unset |

Additional runner features (telemetry presets, secrets bootstrap, admin APIs) will be surfaced directly through this config once the corresponding runner PRs land; the shim already has placeholders so the eventual cut-over is a one-liner re-export.

## CLI Options

CLI flags override the env vars above when provided:

| Flag | Env var |
| --- | --- |
| `--packs-dir` | `PACKS_DIR` |
| `--port` | `PORT` |
| `--secrets-backend` | `SECRETS_BACKEND` |
| `--pack-source` | `PACK_SOURCE` |
| `--pack-index-url` | `PACK_INDEX_URL` |
| `--pack-cache-dir` | `PACK_CACHE_DIR` |
| `--pack-public-key` | `PACK_PUBLIC_KEY` |
| `--pack-refresh-interval` | `PACK_REFRESH_INTERVAL` |
| `--pack-refresh-interval-secs` | `PACK_REFRESH_INTERVAL_SECS` |
| `--tenant-resolver` | `TENANT_RESOLVER` |

## Development Notes

- `make fmt` / `make test` run against `cargo +nightly` because the crate targets Rust 2024 edition.
- `.env` is ignored by Git; `make run` automatically creates it from `.env.example` the first time.
- Historical NATS bridge utilities (`config`, `nats_bridge`, etc.) remain available under `src/` for reference, but new demos should run entirely through the runner host via this bootstrap.
- See `docs/deploy.md` for the Terraform + GitHub Actions deployment flow, required OIDC identities, and how to trigger the `Deploy` workflow.

## Deployment Demo Pack

`examples/deployment/generic-deploy.gtpack` is a provider-agnostic deployment example:
- `kind: deployment` pack with an events flow `deploy_generic_iac` that hands off to a dummy deployment component (`deploy.renderer` kind).
- Component manifest `greentic.deploy.generic.iac` advertises `host.iac` for writing IaC templates and imports `greentic:deploy-plan@1.0.0`.
- Includes a basic configurator flow for the component.
- A compiled stub WASM for `greentic.deploy.generic.iac` is included in the pack, generated from `examples/deployment/stub-deploy-component`. Rebuild it if you tweak the stub:
  - `cd examples/deployment/stub-deploy-component`
  - `cargo build --release --target wasm32-wasip1`
  - copy `target/wasm32-wasip1/release/stub-deploy-component.wasm` over `examples/deployment/generic-deploy.gtpack/components/greentic.deploy.generic.iac.wasm`
- Discovery/run: the default `PACK_INDEX_URL` in `.env.example` already points to `./examples/index.json`, which now includes the deployment demo under tenant `deployment-demo`. To run it locally, set `TENANT_RESOLVER=env` and `TENANT=deployment-demo` (or use a host/header resolver that maps to that tenant) before starting the runner.
