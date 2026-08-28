use tracing::info;

pub async fn extract_symbols(_content: &str, _language: &str) -> anyhow::Result<Vec<String>> {
    info!("Extracting symbols from {} content", _language);
    Ok(vec![])
}
