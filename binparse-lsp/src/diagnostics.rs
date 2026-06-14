use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

use binparse_codegen::CodeGen;

const SOURCE: &str = "binparse";

/// Run the parser, then codegen, over a `.bp` source string and surface any
/// failure as LSP diagnostics. This is the whole wrapper: it reuses the error
/// reporting already produced by `binparse-dsl-parse` and `binparse-codegen`.
pub fn compute(text: &str) -> Vec<Diagnostic> {
    let (defs, errors) = binparse_dsl_parse::parse_str_recover(text);
    if !errors.is_empty() {
        return errors
            .into_iter()
            .map(|e| diagnostic(span_range(text, e.offset), e.message))
            .collect();
    }

    match CodeGen::generate(&defs) {
        Ok(_) => Vec::new(),
        // Codegen errors carry no source span (the AST is span-free), so we
        // attach them to the whole document. The message names the offending
        // struct/field/expression.
        Err(e) => vec![diagnostic(
            Range {
                start: Position::new(0, 0),
                end: end_position(text),
            },
            e.to_string(),
        )],
    }
}

fn diagnostic(range: Range, message: String) -> Diagnostic {
    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some(SOURCE.to_string()),
        message,
        ..Default::default()
    }
}

/// Byte offset into the source -> LSP position (line / UTF-16 column).
fn offset_to_position(text: &str, offset: usize) -> Position {
    let offset = offset.min(text.len());
    let mut line = 0u32;
    let mut col = 0u32;
    for (i, c) in text.char_indices() {
        if i >= offset {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 0;
        } else {
            col += c.len_utf16() as u32;
        }
    }
    Position::new(line, col)
}

fn end_position(text: &str) -> Position {
    offset_to_position(text, text.len())
}

/// A point-ish range covering the single character at `offset` so the squiggle
/// is visible even though the parser only reports a point.
fn span_range(text: &str, offset: usize) -> Range {
    let start = offset_to_position(text, offset);
    let clamped = offset.min(text.len());
    let end_off = text[clamped..]
        .chars()
        .next()
        .map(|c| clamped + c.len_utf8())
        .unwrap_or(clamped);
    Range {
        start,
        end: offset_to_position(text, end_off),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_source_has_no_diagnostics() {
        let src = "struct Packet { len: u8, data: [u8; len] }";
        assert!(compute(src).is_empty());
    }

    #[test]
    fn parse_error_is_located() {
        let src = "struct Packet { len: }";
        let diags = compute(src);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].range.start.line, 0);
    }

    #[test]
    fn semantic_error_surfaced() {
        let src = "struct Packet { data: [u8; missing] }";
        let diags = compute(src);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("missing"));
    }

    #[test]
    fn position_handles_newlines() {
        let src = "struct A {}\nstruct B {";
        let diags = compute(src);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].range.start.line, 1);
    }

    #[test]
    fn multiple_parse_errors_recovered() {
        let src = "struct A { x: }\nstruct B { y: }";
        let diags = compute(src);
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].range.start.line, 0);
        assert_eq!(diags[1].range.start.line, 1);
    }
}
