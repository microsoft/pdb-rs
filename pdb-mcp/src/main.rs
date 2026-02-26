#![forbid(unsafe_code)]

use anyhow::Result;
use rmcp::ServiceExt;

mod format;
mod server;
mod tools;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .init();

    let server = server::PdbMcpServer::new();
    let transport = rmcp::transport::stdio();
    let running = server
        .serve(transport)
        .await
        .inspect_err(|e| tracing::error!("serve error: {e:?}"))?;
    running.waiting().await?;

    Ok(())
}
