use std::path::Path;
use tracing::info;

pub async fn index_directory(path: &Path) -> anyhow::Result<Vec<String>> {
    info!("Indexing directory {:?}", path);
    Ok(vec![])
}
