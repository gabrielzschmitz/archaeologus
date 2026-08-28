#![allow(clippy::missing_errors_doc)]

use std::path::Path;
use tracing::info;

pub fn blame_file(repo_path: &Path, file_path: &str) -> anyhow::Result<Vec<String>> {
    info!("Blaming {file_path} in {repo_path:?}");
    Ok(vec![])
}
