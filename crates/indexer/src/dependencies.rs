#![allow(clippy::missing_errors_doc, clippy::unused_async)]

use tracing::info;

pub async fn analyze_dependencies(content: &str, language: &str) -> anyhow::Result<Vec<String>> {
    info!("Analyzing dependencies in {language} content");
    let _ = content;
    Ok(vec![])
}
