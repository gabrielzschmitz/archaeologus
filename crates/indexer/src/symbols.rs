#![allow(clippy::missing_errors_doc, clippy::unused_async)]

use tracing::info;

pub async fn extract_symbols(content: &str, language: &str) -> anyhow::Result<Vec<String>> {
    info!("Extracting symbols from {language} content");
    let _ = content;
    Ok(vec![])
}
