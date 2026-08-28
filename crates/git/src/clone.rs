use std::path::PathBuf;
use tracing::info;

pub async fn clone_repository(url: &str, dest: &str) -> anyhow::Result<PathBuf> {
    info!("Cloning {} to {}", url, dest);
    let path = PathBuf::from(dest).join("repo");
    Ok(path)
}
