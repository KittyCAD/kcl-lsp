//! Functions for the `kcl` lsp server.

use anyhow::Result;
use tower_lsp::{jsonrpc::Result as RpcResult, lsp_types::*, Client, LanguageServer, LspService, Server as LspServer};

/// The lsp server backend.
#[derive(Debug)]
struct Backend {
    /// The client for the backend.
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> RpcResult<InitializeResult> {
        Ok(InitializeResult::default())
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client.log_message(MessageType::INFO, "server initialized!").await;
    }

    async fn shutdown(&self) -> RpcResult<()> {
        Ok(())
    }
}

/// Run the `kcl` lsp server.
pub async fn run(opts: &crate::Server) -> Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    LspServer::new(stdin, stdout, socket).serve(service).await;

    Ok(())
}
