use tracing::info;

pub async fn analyze_dependencies(_content: &str, _language: &str) -> anyhow::Result<Vec<String>> {
    info!("Analyzing dependencies in {} content", _language);
    Ok(vec![])
}
