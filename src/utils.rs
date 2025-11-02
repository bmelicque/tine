use std::path::{Path, PathBuf};

use crate::parser::parser::ParseError;

pub fn pretty_print_error(error: &ParseError) {
    let span = &error.span;
    let start_pos = span.start_pos();
    let end_pos = span.end_pos();

    let line_str = start_pos.line_col().0;
    let col_start = start_pos.line_col().1;
    let col_end = end_pos.line_col().1;

    let line_text = error
        .span
        .as_str()
        .lines()
        .nth(line_str - 1) // lines are 1-based
        .unwrap_or("");

    println!(
        "\nerror: {}\n --> line {}, column {}\n",
        error.message, line_str, col_start
    );
    println!("{} | {}", line_str, line_text);

    let gutter = " ".repeat(line_str.to_string().len());
    let underline = if col_end > col_start {
        "~".repeat(col_end - col_start)
    } else {
        "^".to_string()
    };

    println!("{} | {}{}", gutter, " ".repeat(col_start - 1), underline);
}

pub fn dummy_span() -> pest::Span<'static> {
    pest::Span::new("_", 0, 0).unwrap()
}

/// Compute a relative path from `base` to `path`.
/// Works even if `path` is outside of `base` (e.g. gives `../../other/file`).
pub fn make_relative(base: &Path, path: &Path) -> PathBuf {
    let base = base.components().collect::<Vec<_>>();
    let path = path.components().collect::<Vec<_>>();

    // Find common prefix length
    let common_prefix_len = base.iter().zip(&path).take_while(|(a, b)| a == b).count();

    // Steps to go up from base to common ancestor
    let mut rel = PathBuf::new();
    for _ in common_prefix_len..base.len() {
        rel.push("..");
    }

    // Steps down to target
    for comp in path.iter().skip(common_prefix_len) {
        rel.push(comp.as_os_str());
    }

    rel
}
