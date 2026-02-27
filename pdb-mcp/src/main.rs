//! # pdb-mcp — MCP Server for PDB Analysis
//!
//! An [MCP](https://modelcontextprotocol.io/) server that gives AI assistants structured,
//! read-only access to Microsoft Program Database (PDB) files. Built on the `ms-pdb` library.
//!
//! The server communicates via JSON-RPC over stdio and exposes ~20 tools for querying symbols,
//! types, modules, streams, and function details. It supports both PDB (MSF) and PDZ (MSFZ)
//! container formats transparently.
//!
//! See `README.md` for the full tool list, installation instructions, and safety guidance.

#![forbid(unsafe_code)]

use anyhow::Result;
use rmcp::ServiceExt;

mod format;
mod server;
mod tools;
pub mod undecorate;

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
