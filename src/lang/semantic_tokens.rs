//! Functions for semantic tokens.

use tower_lsp::{jsonrpc::Result as RpcResult, lsp_types::*, Client, LanguageServer, LspService, Server as LspServer};

/// The type of semantic tokens legend.
pub const LEGEND_TYPE: &[SemanticTokenType] = &[
    SemanticTokenType::FUNCTION,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::STRING,
    SemanticTokenType::COMMENT,
    SemanticTokenType::NUMBER,
    SemanticTokenType::KEYWORD,
    SemanticTokenType::OPERATOR,
    SemanticTokenType::PARAMETER,
];
