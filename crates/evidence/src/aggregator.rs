#![allow(clippy::missing_errors_doc, clippy::unused_async)]

use tracing::info;

pub async fn aggregate_evidence(symbol_id: &str) -> anyhow::Result<Vec<String>> {
    info!("Aggregating evidence for symbol: {symbol_id}");
    Ok(vec![])
}
