//! Functions for the `kcl` lsp server.

use anyhow::Result;
use log::info;
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
use tower_lsp::{jsonrpc::Result as RpcResult, lsp_types::*, Client, LanguageServer, LspService, Server as LspServer};

use crate::lang::semantic_tokens::LEGEND_TYPE;

/// The lsp server backend.
#[derive(Debug)]
struct Backend {
    /// The client for the backend.
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> RpcResult<InitializeResult> {
        log::info!("initialize params: {:?}", params);
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                inlay_hint_provider: Some(OneOf::Left(true)),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string()]),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensRegistrationOptions(
                    SemanticTokensRegistrationOptions {
                        text_document_registration_options: {
                            TextDocumentRegistrationOptions {
                                document_selector: Some(vec![DocumentFilter {
                                    language: Some("nrs".to_string()),
                                    scheme: Some("file".to_string()),
                                    pattern: None,
                                }]),
                            }
                        },
                        semantic_tokens_options: SemanticTokensOptions {
                            work_done_progress_options: WorkDoneProgressOptions::default(),
                            legend: SemanticTokensLegend {
                                token_types: LEGEND_TYPE.into(),
                                token_modifiers: vec![],
                            },
                            range: Some(true),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                        },
                        static_registration_options: StaticRegistrationOptions::default(),
                    },
                )),
                // definition: Some(GotoCapability::default()),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, params: InitializedParams) {
        log::info!("server initialized: {:?}", params);
        self.client.log_message(MessageType::INFO, "server initialized!").await;
    }

    async fn shutdown(&self) -> RpcResult<()> {
        Ok(())
    }

    async fn hover(&self, params: HoverParams) -> RpcResult<Option<Hover>> {
        log::info!("hover: {:?}", params);
        todo!()
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> RpcResult<Option<GotoDefinitionResponse>> {
        log::info!("goto definition: {:?}", params);
        todo!()
    }

    async fn references(&self, params: ReferenceParams) -> RpcResult<Option<Vec<Location>>> {
        log::info!("references: {:?}", params);
        todo!()
    }

    async fn semantic_tokens_full(&self, params: SemanticTokensParams) -> RpcResult<Option<SemanticTokensResult>> {
        log::info!("semantic tokens full: {:?}", params);
        todo!()
    }

    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> RpcResult<Option<SemanticTokensRangeResult>> {
        log::info!("semantic tokens range: {:?}", params);
        todo!()
    }

    async fn inlay_hint(&self, params: tower_lsp::lsp_types::InlayHintParams) -> RpcResult<Option<Vec<InlayHint>>> {
        log::info!("inlay hint: {:?}", params);
        todo!()
    }

    async fn completion(&self, params: CompletionParams) -> RpcResult<Option<CompletionResponse>> {
        log::info!("completion: {:?}", params);
        todo!()
    }

    async fn rename(&self, params: RenameParams) -> RpcResult<Option<WorkspaceEdit>> {
        log::info!("rename: {:?}", params);
        todo!()
    }
}

/// Run the `kcl` lsp server.
pub async fn run(opts: &crate::Server) -> Result<()> {
    // For Cloud run & ctrl+c, shutdown gracefully.
    // "The main process inside the container will receive SIGTERM, and after a grace period,
    // SIGKILL."
    // Regsitering SIGKILL here will panic at runtime, so let's avoid that.
    let mut signals = Signals::new([SIGINT, SIGTERM])?;

    tokio::spawn(async move {
        for sig in signals.forever() {
            info!("received signal: {:?}", sig);
            info!("triggering cleanup...");

            // Exit the process.
            info!("all clean, exiting!");
            std::process::exit(0);
        }
    });

    let (service, socket) = LspService::new(|client| Backend { client });

    if opts.stdio {
        // Listen on stdin and stdout.
        log::info!("Listening on stdin/stdout");
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        LspServer::new(stdin, stdout, socket).serve(service).await;
    } else {
        // Listen on a tcp stream.
        log::info!("Listening on {}", opts.socket);
        let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", opts.socket)).await?;
        let (stream, _) = listener.accept().await?;
        let (read, write) = tokio::io::split(stream);
        LspServer::new(read, write, socket).serve(service).await;
    }

    Ok(())
}
