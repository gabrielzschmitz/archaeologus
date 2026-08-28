#![allow(clippy::missing_errors_doc)]

use std::path::Path;
use tracing::info;

pub fn index_directory(path: &Path) -> anyhow::Result<Vec<String>> {
    info!("Indexing directory {path:?}");
    Ok(vec![])
}
