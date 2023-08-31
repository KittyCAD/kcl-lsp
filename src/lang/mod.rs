//! Functions for the language server protocol.

use kcl_lib::executor::SourceRange;
use tower_lsp::lsp_types::Position;

pub mod semantic_tokens;

fn source_range_to_position(code: &str, source_range: SourceRange) -> Position {
    // Calculate the line and column of the error from the source range.
    let line = code[..source_range.0[0]].lines().count();
    let column = code[..source_range.0[0]]
        .lines()
        .last()
        .map(|l| l.len())
        .unwrap_or_default();

    Position {
        line: line as u32,
        character: column as u32,
    }
}
