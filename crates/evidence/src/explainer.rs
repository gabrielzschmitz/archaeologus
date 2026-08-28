#![allow(clippy::missing_errors_doc, clippy::unused_async)]

use tracing::info;

pub async fn explain_symbol(symbol_name: &str) -> anyhow::Result<String> {
    info!("Explaining symbol: {symbol_name}");
    Ok(format!("Explanation for {symbol_name}"))
}
