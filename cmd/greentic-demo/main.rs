use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Parser;
use dotenvy::dotenv;
use greentic_demo::loader::load_packs;
use greentic_demo::runner_shim::{self, RunnerConfig};

#[cfg(not(any(feature = "runner-shim", feature = "use-runner-api")))]
compile_error!("either runner-shim or use-runner-api must be enabled");
#[cfg(feature = "use-runner-api")]
use greentic_config::ConfigResolver;

#[derive(Debug, Parser)]
#[command(author, version, about = "greentic demo server", long_about = None)]
struct CliArgs {
    /// Directory containing tenant bindings.yaml files.
    #[arg(long)]
    packs_dir: Option<PathBuf>,

    /// HTTP listener port exposed by the runner host.
    #[arg(long)]
    port: Option<u16>,

    /// Secrets backend hint (env, aws, gcp, azure).
    #[arg(long)]
    secrets_backend: Option<String>,

    /// Pack resolver scheme (fs, http, oci, s3, gcs, azblob).
    #[arg(long)]
    pack_source: Option<String>,

    /// Local path or URL to index.json.
    #[arg(long)]
    pack_index_url: Option<String>,

    /// Content-addressed cache root.
    #[arg(long)]
    pack_cache_dir: Option<String>,

    /// Optional Ed25519 key for signed packs.
    #[arg(long)]
    pack_public_key: Option<String>,

    /// Polling interval (e.g. 30s, 5m).
    #[arg(long, conflicts_with = "pack_refresh_interval_secs")]
    pack_refresh_interval: Option<String>,

    /// Polling interval in seconds.
    #[arg(long, conflicts_with = "pack_refresh_interval")]
    pack_refresh_interval_secs: Option<u64>,

    /// Routing strategy (host, header, jwt, env).
    #[arg(long)]
    tenant_resolver: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let args = CliArgs::parse();
    apply_cli_overrides(&args);
    init_tracing();
    let bindings = discover_binding_files()?;
    let cfg = build_runner_config(bindings)?;
    runner_shim::run(cfg).await
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt::try_init();
}

fn apply_cli_overrides(args: &CliArgs) {
    if let Some(value) = &args.packs_dir {
        set_env_var("PACKS_DIR", value);
    }
    if let Some(value) = args.port {
        set_env_var("PORT", value.to_string());
    }
    if let Some(value) = &args.secrets_backend {
        set_env_var("SECRETS_BACKEND", value);
    }
    if let Some(value) = &args.pack_source {
        set_env_var("PACK_SOURCE", value);
    }
    if let Some(value) = &args.pack_index_url {
        set_env_var("PACK_INDEX_URL", value);
    }
    if let Some(value) = &args.pack_cache_dir {
        set_env_var("PACK_CACHE_DIR", value);
    }
    if let Some(value) = &args.pack_public_key {
        set_env_var("PACK_PUBLIC_KEY", value);
    }
    if let Some(value) = &args.pack_refresh_interval {
        set_env_var("PACK_REFRESH_INTERVAL", value);
    }
    if let Some(value) = args.pack_refresh_interval_secs {
        set_env_var("PACK_REFRESH_INTERVAL_SECS", value.to_string());
    }
    if let Some(value) = &args.tenant_resolver {
        set_env_var("TENANT_RESOLVER", value);
    }
}

fn set_env_var(key: &str, value: impl AsRef<OsStr>) {
    // Safe during single-threaded startup before any child threads spawn.
    unsafe {
        env::set_var(key, value);
    }
}

fn discover_binding_files() -> Result<Vec<PathBuf>> {
    let packs_dir = env::var("PACKS_DIR").unwrap_or_else(|_| "./packs".into());
    let packs_path = Path::new(&packs_dir);
    let tenants = load_packs(packs_path)
        .with_context(|| format!("failed to load tenant packs from {packs_dir}"))?;

    if tenants.is_empty() {
        bail!("no tenant bindings found in {packs_dir}; add at least one pack");
    }

    Ok(tenants.into_iter().map(|pack| pack.bindings_path).collect())
}

#[cfg(feature = "use-runner-api")]
fn build_runner_config(bindings: Vec<PathBuf>) -> Result<RunnerConfig> {
    let resolved_cfg = ConfigResolver::new()
        .with_allow_dev(true)
        .with_allow_network(true)
        .load()
        .context("failed to resolve greentic configuration")?;
    RunnerConfig::from_config(resolved_cfg, bindings).context("failed to build runner config")
}

#[cfg(not(feature = "use-runner-api"))]
fn build_runner_config(bindings: Vec<PathBuf>) -> Result<RunnerConfig> {
    RunnerConfig::from_env(bindings)
}
