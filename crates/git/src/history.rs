#![allow(clippy::missing_errors_doc)]

use std::path::Path;
use tracing::info;

pub fn walk_commits(repo_path: &Path) -> anyhow::Result<Vec<String>> {
    info!("Walking commits in {repo_path:?}");
    Ok(vec![])
}
