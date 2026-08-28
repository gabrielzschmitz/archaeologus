#![allow(clippy::missing_errors_doc)]

use std::path::PathBuf;
use tracing::info;

pub fn clone_repository(url: &str, dest: &str) -> anyhow::Result<PathBuf> {
    info!("Cloning {url} to {dest}");
    let path = PathBuf::from(dest).join("repo");
    Ok(path)
}
