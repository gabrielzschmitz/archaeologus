use std::path::Path;
use tracing::info;

pub async fn walk_commits(repo_path: &Path) -> anyhow::Result<Vec<String>> {
    info!("Walking commits in {:?}", repo_path);
    Ok(vec![])
}
