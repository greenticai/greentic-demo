use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use dotenvy::dotenv;
use greentic_demo::loader::load_packs;
use greentic_demo::runner_shim::{self, RunnerConfig};

#[cfg(not(any(feature = "runner-shim", feature = "use-runner-api")))]
compile_error!("either runner-shim or use-runner-api must be enabled");
#[cfg(feature = "use-runner-api")]
use greentic_config::ConfigResolver;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_tracing();
    let bindings = discover_binding_files()?;
    let cfg = build_runner_config(bindings)?;
    runner_shim::run(cfg).await
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt::try_init();
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
