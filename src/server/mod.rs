//! Functions for the `kcl` lsp server.

use std::sync::Arc;

use anyhow::Result;
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
use tower_lsp::{jsonrpc::Result as RpcResult, lsp_types::*, Client, LanguageServer, LspService, Server as LspServer};

use crate::lang::semantic_tokens::LEGEND_TYPE;

/// The lsp server backend.
struct Backend {
    /// The client for the backend.
    client: Client,
    /// The stdlib completions for the language.
    stdlib_completions: Vec<CompletionItem>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> RpcResult<InitializeResult> {
        self.client
            .log_message(MessageType::INFO, format!("initialize: {:?}", params))
            .await;

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string()]),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, format!("initialized: {:?}", params))
            .await;
    }

    async fn shutdown(&self) -> RpcResult<()> {
        self.client.log_message(MessageType::INFO, "shutdown".to_string()).await;
        Ok(())
    }

    async fn hover(&self, params: HoverParams) -> RpcResult<Option<Hover>> {
        self.client
            .log_message(MessageType::INFO, format!("hover: {:?}", params))
            .await;
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String("You're hovering!".to_string())),
            range: None,
        }))
    }

    async fn completion(&self, params: CompletionParams) -> RpcResult<Option<CompletionResponse>> {
        self.client
            .log_message(MessageType::INFO, format!("completion: {:?}", params))
            .await;

        let mut completions = vec![
            CompletionItem::new_simple("|>".to_string(), "A pipe operator.".to_string()),
            CompletionItem::new_simple("let".to_string(), "A let binding.".to_string()),
            CompletionItem::new_simple("const".to_string(), "A const binding.".to_string()),
            CompletionItem::new_simple("show".to_string(), "Show a model.".to_string()),
        ];
        completions.extend(self.stdlib_completions.clone());
        Ok(Some(CompletionResponse::Array(completions)))
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> RpcResult<Option<GotoDefinitionResponse>> {
        self.client
            .log_message(MessageType::INFO, format!("goto_definition: {:?}", params))
            .await;
        todo!();
        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> RpcResult<Option<Vec<Location>>> {
        self.client
            .log_message(MessageType::INFO, format!("references: {:?}", params))
            .await;
        todo!();
        Ok(None)
    }

    async fn semantic_tokens_full(&self, params: SemanticTokensParams) -> RpcResult<Option<SemanticTokensResult>> {
        self.client
            .log_message(MessageType::INFO, format!("semantic_tokens_full: {:?}", params))
            .await;
        todo!();
        Ok(None)
    }

    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> RpcResult<Option<SemanticTokensRangeResult>> {
        self.client
            .log_message(MessageType::INFO, format!("semantic_tokens_range: {:?}", params))
            .await;
        todo!();
        Ok(None)
    }

    async fn inlay_hint(&self, params: tower_lsp::lsp_types::InlayHintParams) -> RpcResult<Option<Vec<InlayHint>>> {
        self.client
            .log_message(MessageType::INFO, format!("inlay_hint: {:?}", params))
            .await;
        todo!();
        Ok(None)
    }

    async fn rename(&self, params: RenameParams) -> RpcResult<Option<WorkspaceEdit>> {
        self.client
            .log_message(MessageType::INFO, format!("rename: {:?}", params))
            .await;
        todo!();
        Ok(None)
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
            log::info!("received signal: {:?}", sig);
            log::info!("triggering cleanup...");

            // Exit the process.
            log::info!("all clean, exiting!");
            std::process::exit(0);
        }
    });

    let stdlib = kcl_lib::std::StdLib::new();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        stdlib_completions: get_completions_from_stdlib(&stdlib),
    });

    if opts.stdio {
        // Listen on stdin and stdout.
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        LspServer::new(stdin, stdout, socket).serve(service).await;
    } else {
        // Listen on a tcp stream.
        let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", opts.socket)).await?;
        let (stream, _) = listener.accept().await?;
        let (read, write) = tokio::io::split(stream);
        LspServer::new(read, write, socket).serve(service).await;
    }

    Ok(())
}

fn get_completions_from_stdlib(stdlib: &kcl_lib::std::StdLib) -> Vec<CompletionItem> {
    let mut fns = Vec::new();

    for internal_fn in &stdlib.internal_fn_names {
        fns.push(CompletionItem {
            label: internal_fn.name(),
            label_details: Some(CompletionItemLabelDetails {
                detail: Some(internal_fn.fn_signature().replace(&internal_fn.name(), "")),
                description: None,
            }),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: None,
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: if !internal_fn.description().is_empty() {
                    format!("{}\n\n{}", internal_fn.summary(), internal_fn.description())
                } else {
                    internal_fn.summary()
                },
            })),
            deprecated: Some(internal_fn.deprecated()),
            preselect: None,
            sort_text: None,
            filter_text: None,
            insert_text: None,
            insert_text_format: None,
            insert_text_mode: None,
            text_edit: None,
            additional_text_edits: None,
            command: None,
            commit_characters: None,
            data: None,
            tags: None,
        });
    }

    fns
}
