//! MCP server of `swhy` over stdio.
//!
//! E0: the server answers the handshake and `tools/list` with an empty list.
//! The four tools of docs/TZ.md ("Интерфейсы") are added at E14.
//!
//! Nothing in this crate prints: the standard output belongs to the rmcp
//! transport and carries JSON-RPC only.

use rmcp::{
    ServerHandler, ServiceExt,
    model::{Implementation, ServerCapabilities, ServerConfig},
    transport::stdio,
};

/// Name the server reports in `serverInfo.name`.
pub const SERVER_NAME: &str = "swhy";

/// The `swhy` MCP server.
#[derive(Debug, Clone, Default)]
pub struct SwhyServer;

impl ServerHandler for SwhyServer {
    // Without this override the server would introduce itself as `rmcp`.
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(SERVER_NAME, swhy_core::VERSION))
    }
}

/// Error of [`run_stdio`].
pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Serves MCP over the stdio transport until the client closes the connection.
pub async fn serve_stdio() -> Result<(), Error> {
    let service = SwhyServer.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

/// Starts a tokio runtime and runs [`serve_stdio`] on it. Entry point of
/// `swhy mcp`.
pub fn run_stdio() -> Result<(), Error> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(serve_stdio())
}
