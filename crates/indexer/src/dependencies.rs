use tracing::info;

pub async fn analyze_dependencies(content: &str, language: &str) -> anyhow::Result<Vec<String>> {
    info!("Analyzing dependencies in {} content", language);
    Ok(vec![])
}
