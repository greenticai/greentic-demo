use std::fs;
use std::path::{Path, PathBuf};

use crate::path_safety::normalize_under_root;
use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct TenantPack {
    pub tenant: String,
    pub index_path: PathBuf,
    pub bindings_path: PathBuf,
}

pub fn load_packs(packs_dir: &Path) -> Result<Vec<TenantPack>> {
    let root = packs_dir
        .canonicalize()
        .with_context(|| format!("packs directory {} does not exist", packs_dir.display()))?;
    let mut packs = Vec::new();
    let entries =
        fs::read_dir(&root).with_context(|| format!("packs directory {root:?} does not exist"))?;

    for entry in entries {
        let entry = match entry {
            Ok(value) => value,
            Err(err) => {
                tracing::warn!(error = %err, "skipping unreadable directory entry");
                continue;
            }
        };

        let tenant_name = match entry.file_name().into_string() {
            Ok(name) => name,
            Err(os) => {
                tracing::warn!(entry = ?os, "unable to derive tenant name; skipping");
                continue;
            }
        };

        let path = match normalize_under_root(&root, Path::new(&tenant_name)) {
            Ok(path) => path,
            Err(err) => {
                tracing::warn!(tenant = %tenant_name, error = %err, "skipping pack outside root");
                continue;
            }
        };

        if !path.is_dir() {
            continue;
        }

        let index_path = path.join("index.ygtc");
        if !index_path.exists() {
            tracing::warn!(tenant = %tenant_name, path = ?index_path, "tenant pack missing index.ygtc");
            continue;
        }

        let bindings_path = match discover_bindings(&path) {
            Some(path) => path,
            None => {
                tracing::error!(
                    tenant = %tenant_name,
                    dir = %path.display(),
                    "bindings.yaml not found; please add one per tenant"
                );
                continue;
            }
        };

        packs.push(TenantPack {
            tenant: tenant_name,
            index_path,
            bindings_path,
        });
    }

    tracing::info!(count = packs.len(), base = ?packs_dir, "packs discovered");
    Ok(packs)
}

fn discover_bindings(pack_dir: &Path) -> Option<PathBuf> {
    const CANDIDATES: &[&str] = &["bindings.yaml", "bindings.yml"];

    for candidate in CANDIDATES {
        let path = pack_dir.join(candidate);
        if path.exists() {
            return Some(path);
        }
    }

    None
}
