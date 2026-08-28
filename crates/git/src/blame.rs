use std::path::Path;
use tracing::info;

pub async fn blame_file(repo_path: &Path, file_path: &str) -> anyhow::Result<Vec<String>> {
    info!("Blaming {} in {:?}", file_path, repo_path);
    Ok(vec![])
}
