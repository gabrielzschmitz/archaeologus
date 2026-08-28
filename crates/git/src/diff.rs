use std::path::Path;
use tracing::info;

pub async fn diff_commit(repo_path: &Path, sha: &str) -> anyhow::Result<Vec<String>> {
    info!("Diffing {} in {:?}", sha, repo_path);
    Ok(vec![])
}
