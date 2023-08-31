//! Functions for the `kcl` lsp server.

use std::collections::HashMap;

use anyhow::Result;
use dashmap::DashMap;
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
    // TODO: fix last string.
    ast_map: DashMap<String, HashMap<String, String>>,
    // TODO: fix last string.
    document_map: DashMap<String, String>,
    // TODO: fix last string.
    semantic_token_map: DashMap<String, Vec<String>>,
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

    async fn hover(&self, _: HoverParams) -> RpcResult<Option<Hover>> {
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String("You're hovering!".to_string())),
            range: None,
        }))
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> RpcResult<Option<GotoDefinitionResponse>> {
        let definition = async {
            let uri = params.text_document_position_params.text_document.uri;
            let ast = self.ast_map.get(uri.as_str())?;
            let rope = self.document_map.get(uri.as_str())?;

            let position = params.text_document_position_params.position;
            let char = rope.try_line_to_char(position.line as usize).ok()?;
            let offset = char + position.character as usize;
            // self.client.log_message(MessageType::INFO, &format!("{:#?}, {}", ast.value(), offset)).await;
            let span = get_definition(&ast, offset);
            self.client
                .log_message(MessageType::INFO, &format!("{:?}, ", span))
                .await;
            span.and_then(|(_, range)| {
                let start_position = offset_to_position(range.start, &rope)?;
                let end_position = offset_to_position(range.end, &rope)?;

                let range = Range::new(start_position, end_position);

                Some(GotoDefinitionResponse::Scalar(Location::new(uri, range)))
            })
        }
        .await;
        Ok(definition)
    }

    async fn references(&self, params: ReferenceParams) -> RpcResult<Option<Vec<Location>>> {
        let reference_list = || -> Option<Vec<Location>> {
            let uri = params.text_document_position.text_document.uri;
            let ast = self.ast_map.get(&uri.to_string())?;
            let rope = self.document_map.get(&uri.to_string())?;

            let position = params.text_document_position.position;
            let char = rope.try_line_to_char(position.line as usize).ok()?;
            let offset = char + position.character as usize;
            let reference_list = get_reference(&ast, offset, false);
            let ret = reference_list
                .into_iter()
                .filter_map(|(_, range)| {
                    let start_position = offset_to_position(range.start, &rope)?;
                    let end_position = offset_to_position(range.end, &rope)?;

                    let range = Range::new(start_position, end_position);

                    Some(Location::new(uri.clone(), range))
                })
                .collect::<Vec<_>>();
            Some(ret)
        }();
        Ok(reference_list)
    }

    async fn semantic_tokens_full(&self, params: SemanticTokensParams) -> RpcResult<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri.to_string();
        self.client.log_message(MessageType::LOG, "semantic_token_full").await;
        let semantic_tokens = || -> Option<Vec<SemanticToken>> {
            let mut im_complete_tokens = self.semantic_token_map.get_mut(&uri)?;
            let rope = self.document_map.get(&uri)?;
            let ast = self.ast_map.get(&uri)?;
            let extends_tokens = semantic_token_from_ast(&ast);
            im_complete_tokens.extend(extends_tokens);
            im_complete_tokens.sort_by(|a, b| a.start.cmp(&b.start));
            let mut pre_line = 0;
            let mut pre_start = 0;
            let semantic_tokens = im_complete_tokens
                .iter()
                .filter_map(|token| {
                    let line = rope.try_byte_to_line(token.start).ok()? as u32;
                    let first = rope.try_line_to_char(line as usize).ok()? as u32;
                    let start = rope.try_byte_to_char(token.start).ok()? as u32 - first;
                    let delta_line = line - pre_line;
                    let delta_start = if delta_line == 0 { start - pre_start } else { start };
                    let ret = Some(SemanticToken {
                        delta_line,
                        delta_start,
                        length: token.length as u32,
                        token_type: token.token_type as u32,
                        token_modifiers_bitset: 0,
                    });
                    pre_line = line;
                    pre_start = start;
                    ret
                })
                .collect::<Vec<_>>();
            Some(semantic_tokens)
        }();
        if let Some(semantic_token) = semantic_tokens {
            return Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
                result_id: None,
                data: semantic_token,
            })));
        }
        Ok(None)
    }

    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> RpcResult<Option<SemanticTokensRangeResult>> {
        let uri = params.text_document.uri.to_string();
        let semantic_tokens = || -> Option<Vec<SemanticToken>> {
            let im_complete_tokens = self.semantic_token_map.get(&uri)?;
            let rope = self.document_map.get(&uri)?;
            let mut pre_line = 0;
            let mut pre_start = 0;
            let semantic_tokens = im_complete_tokens
                .iter()
                .filter_map(|token| {
                    let line = rope.try_byte_to_line(token.start).ok()? as u32;
                    let first = rope.try_line_to_char(line as usize).ok()? as u32;
                    let start = rope.try_byte_to_char(token.start).ok()? as u32 - first;
                    let ret = Some(SemanticToken {
                        delta_line: line - pre_line,
                        delta_start: if start >= pre_start { start - pre_start } else { start },
                        length: token.length as u32,
                        token_type: token.token_type as u32,
                        token_modifiers_bitset: 0,
                    });
                    pre_line = line;
                    pre_start = start;
                    ret
                })
                .collect::<Vec<_>>();
            Some(semantic_tokens)
        }();
        if let Some(semantic_token) = semantic_tokens {
            return Ok(Some(SemanticTokensRangeResult::Tokens(SemanticTokens {
                result_id: None,
                data: semantic_token,
            })));
        }
        Ok(None)
    }

    async fn inlay_hint(&self, params: tower_lsp::lsp_types::InlayHintParams) -> RpcResult<Option<Vec<InlayHint>>> {
        self.client.log_message(MessageType::INFO, "inlay hint").await;
        let uri = &params.text_document.uri;
        let mut hashmap = HashMap::new();
        if let Some(ast) = self.ast_map.get(uri.as_str()) {
            ast.iter().for_each(|(_, v)| {
                type_inference(&v.body, &mut hashmap);
            });
        }

        let document = match self.document_map.get(uri.as_str()) {
            Some(rope) => rope,
            None => return Ok(None),
        };
        let inlay_hint_list = hashmap
            .into_iter()
            .map(|(k, v)| {
                (
                    k.start,
                    k.end,
                    match v {
                        nrs_language_server::chumsky::Value::Null => "null".to_string(),
                        nrs_language_server::chumsky::Value::Bool(_) => "bool".to_string(),
                        nrs_language_server::chumsky::Value::Num(_) => "number".to_string(),
                        nrs_language_server::chumsky::Value::Str(_) => "string".to_string(),
                        nrs_language_server::chumsky::Value::List(_) => "[]".to_string(),
                        nrs_language_server::chumsky::Value::Func(_) => v.to_string(),
                    },
                )
            })
            .filter_map(|item| {
                // let start_position = offset_to_position(item.0, document)?;
                let end_position = offset_to_position(item.1, &document)?;
                let inlay_hint = InlayHint {
                    text_edits: None,
                    tooltip: None,
                    kind: Some(InlayHintKind::TYPE),
                    padding_left: None,
                    padding_right: None,
                    data: None,
                    position: end_position,
                    label: InlayHintLabel::LabelParts(vec![InlayHintLabelPart {
                        value: item.2,
                        tooltip: None,
                        location: Some(Location {
                            uri: params.text_document.uri.clone(),
                            range: Range {
                                start: Position::new(0, 4),
                                end: Position::new(0, 5),
                            },
                        }),
                        command: None,
                    }]),
                };
                Some(inlay_hint)
            })
            .collect::<Vec<_>>();

        Ok(Some(inlay_hint_list))
    }

    async fn completion(&self, params: CompletionParams) -> RpcResult<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let completions = || -> Option<Vec<CompletionItem>> {
            let rope = self.document_map.get(&uri.to_string())?;
            let ast = self.ast_map.get(&uri.to_string())?;
            let char = rope.try_line_to_char(position.line as usize).ok()?;
            let offset = char + position.character as usize;
            let completions = completion(&ast, offset);
            let mut ret = Vec::with_capacity(completions.len());
            for (_, item) in completions {
                match item {
                    nrs_language_server::completion::ImCompleteCompletionItem::Variable(var) => {
                        ret.push(CompletionItem {
                            label: var.clone(),
                            insert_text: Some(var.clone()),
                            kind: Some(CompletionItemKind::VARIABLE),
                            detail: Some(var),
                            ..Default::default()
                        });
                    }
                    nrs_language_server::completion::ImCompleteCompletionItem::Function(name, args) => {
                        ret.push(CompletionItem {
                            label: name.clone(),
                            kind: Some(CompletionItemKind::FUNCTION),
                            detail: Some(name.clone()),
                            insert_text: Some(format!(
                                "{}({})",
                                name,
                                args.iter()
                                    .enumerate()
                                    .map(|(index, item)| { format!("${{{}:{}}}", index + 1, item) })
                                    .collect::<Vec<_>>()
                                    .join(",")
                            )),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            ..Default::default()
                        });
                    }
                }
            }
            Some(ret)
        }();
        Ok(completions.map(CompletionResponse::Array))
    }

    async fn rename(&self, params: RenameParams) -> RpcResult<Option<WorkspaceEdit>> {
        let workspace_edit = || -> Option<WorkspaceEdit> {
            let uri = params.text_document_position.text_document.uri;
            let ast = self.ast_map.get(&uri.to_string())?;
            let rope = self.document_map.get(&uri.to_string())?;

            let position = params.text_document_position.position;
            let char = rope.try_line_to_char(position.line as usize).ok()?;
            let offset = char + position.character as usize;
            let reference_list = get_reference(&ast, offset, true);
            let new_name = params.new_name;
            if !reference_list.is_empty() {
                let edit_list = reference_list
                    .into_iter()
                    .filter_map(|(_, range)| {
                        let start_position = offset_to_position(range.start, &rope)?;
                        let end_position = offset_to_position(range.end, &rope)?;
                        Some(TextEdit::new(
                            Range::new(start_position, end_position),
                            new_name.clone(),
                        ))
                    })
                    .collect::<Vec<_>>();
                let mut map = HashMap::new();
                map.insert(uri, edit_list);
                let workspace_edit = WorkspaceEdit::new(map);
                Some(workspace_edit)
            } else {
                None
            }
        }();
        Ok(workspace_edit)
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

    let (service, socket) = LspService::new(|client| Backend {
        client,
        ast_map: DashMap::new(),
        document_map: DashMap::new(),
        semantic_token_map: DashMap::new(),
    });

    if opts.stdio {
        // Listen on stdin and stdout.
        log::info!("Listening on stdin/stdout");
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        LspServer::new(stdin, stdout, socket).serve(service).await;
    } else {
        // Listen on a tcp stream.
        log::info!("Listening on {}", opts.address);
        let listener = tokio::net::TcpListener::bind(&opts.address).await?;
        let (stream, _) = listener.accept().await?;
        let (read, write) = tokio::io::split(stream);
        LspServer::new(read, write, socket).serve(service).await;
    }

    Ok(())
}
