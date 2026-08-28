#![allow(clippy::missing_errors_doc)]

use std::path::Path;
use tracing::info;

pub fn diff_commit(repo_path: &Path, sha: &str) -> anyhow::Result<Vec<String>> {
    info!("Diffing {sha} in {repo_path:?}");
    Ok(vec![])
}
