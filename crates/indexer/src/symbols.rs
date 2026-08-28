use tracing::info;

pub async fn extract_symbols(content: &str, language: &str) -> anyhow::Result<Vec<String>> {
    info!("Extracting symbols from {} content", language);
    Ok(vec![])
}
