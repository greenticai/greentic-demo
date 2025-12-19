#[cfg(feature = "runner-shim")]
pub mod config;
#[cfg(feature = "runner-shim")]
pub mod health;
pub mod loader;
#[cfg(feature = "runner-shim")]
pub mod logging;
#[cfg(feature = "runner-shim")]
pub mod nats_bridge;
pub mod path_safety;
#[cfg(feature = "runner-shim")]
pub mod runner_bridge;
#[cfg(any(feature = "runner-shim", feature = "use-runner-api"))]
pub mod runner_shim;
#[cfg(feature = "runner-shim")]
pub mod secrets;
#[cfg(feature = "runner-shim")]
pub mod telemetry;
#[cfg(feature = "runner-shim")]
pub mod types;

#[cfg(feature = "runner-shim")]
pub use config::{AppConfig, CliArgs, Mode, SubjectConfig};
pub use loader::{TenantPack, load_packs};
