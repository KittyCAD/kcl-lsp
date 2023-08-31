//! Functions for the `kcl` lsp server.

use anyhow::Result;
use dashmap::DashMap;
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
use tower_lsp::{jsonrpc::Result as RpcResult, lsp_types::*, Client, LanguageServer, LspService, Server as LspServer};

/// The lsp server backend.
struct Backend {
    /// The client for the backend.
    client: Client,
    /// The stdlib completions for the language.
    stdlib_completions: Vec<CompletionItem>,
    /// Token maps.
    token_map: DashMap<String, Vec<kcl_lib::tokeniser::Token>>,
    /// AST maps.
    ast_map: DashMap<String, kcl_lib::abstract_syntax_tree_types::Program>,
}

impl Backend {
    async fn on_change(&self, params: TextDocumentItem) {
        self.client
            .log_message(MessageType::INFO, format!("on_change: {:?}", params))
            .await;
        // Lets update the tokens.
        let tokens = kcl_lib::tokeniser::lexer(&params.text);
        self.token_map.insert(params.uri.to_string(), tokens.clone());
        // Lets update the ast.
        let ast = match kcl_lib::parser::abstract_syntax_tree(&tokens) {
            Ok(ast) => ast,
            Err(e) => {
                // TODO: put an error on the line.
                self.client
                    .log_message(MessageType::ERROR, format!("Error parsing: {:?}", e))
                    .await;
                return;
            }
        };
        self.ast_map.insert(params.uri.to_string(), ast);
    }

    async fn completions_get_variables_from_ast(&self, file_name: &str) -> Vec<CompletionItem> {
        let mut completions = vec![];

        let ast = match self.ast_map.get(file_name) {
            Some(ast) => ast,
            None => return completions,
        };

        for item in &ast.body {
            match item {
                kcl_lib::abstract_syntax_tree_types::BodyItem::ExpressionStatement(_) => continue,
                kcl_lib::abstract_syntax_tree_types::BodyItem::ReturnStatement(_) => continue,
                kcl_lib::abstract_syntax_tree_types::BodyItem::VariableDeclaration(variable) => {
                    // We only want to complete variables.
                    for declaration in &variable.declarations {
                        completions.push(CompletionItem {
                            label: declaration.id.name.to_string(),
                            label_details: None,
                            kind: Some(match variable.kind {
                                kcl_lib::abstract_syntax_tree_types::VariableKind::Let => CompletionItemKind::VARIABLE,
                                kcl_lib::abstract_syntax_tree_types::VariableKind::Const => {
                                    CompletionItemKind::CONSTANT
                                }
                                kcl_lib::abstract_syntax_tree_types::VariableKind::Var => CompletionItemKind::VARIABLE,
                                kcl_lib::abstract_syntax_tree_types::VariableKind::Fn => CompletionItemKind::FUNCTION,
                            }),
                            detail: Some(variable.kind.to_string()),
                            documentation: None,
                            deprecated: None,
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
                }
            }
        }

        completions
    }
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
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
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

    async fn did_change_workspace_folders(&self, _: DidChangeWorkspaceFoldersParams) {
        self.client
            .log_message(MessageType::INFO, "workspace folders changed!")
            .await;
    }

    async fn did_change_configuration(&self, _: DidChangeConfigurationParams) {
        self.client
            .log_message(MessageType::INFO, "configuration changed!")
            .await;
    }

    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {
        self.client
            .log_message(MessageType::INFO, "watched files have changed!")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_change(TextDocumentItem {
            uri: params.text_document.uri,
            text: params.text_document.text,
            version: params.text_document.version,
            language_id: params.text_document.language_id,
        })
        .await
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        self.on_change(TextDocumentItem {
            uri: params.text_document.uri,
            text: std::mem::take(&mut params.content_changes[0].text),
            version: params.text_document.version,
            language_id: Default::default(),
        })
        .await
    }

    async fn did_save(&self, _: DidSaveTextDocumentParams) {
        self.client.log_message(MessageType::INFO, "file saved!").await;
    }

    async fn did_close(&self, _: DidCloseTextDocumentParams) {
        self.client.log_message(MessageType::INFO, "file closed!").await;
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

        // Get our variables from our AST to include in our completions.
        completions.extend(
            self.completions_get_variables_from_ast(params.text_document_position.text_document.uri.as_ref())
                .await,
        );

        Ok(Some(CompletionResponse::Array(completions)))
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
        token_map: Default::default(),
        ast_map: Default::default(),
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
