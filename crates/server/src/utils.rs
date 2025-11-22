use tower_lsp::lsp_types::{Position, Range};
use url::Url;

pub fn normalize_file_url(url: &Url) -> Option<Url> {
    if url.scheme() != "file" {
        return Some(url.clone());
    }

    Url::from_file_path(std::fs::canonicalize(url.to_file_path().unwrap()).unwrap()).ok()
}

pub fn position_in_range(pos: Position, range: &Range) -> bool {
    (pos.line > range.start.line
        || (pos.line == range.start.line && pos.character >= range.start.character))
        && (pos.line < range.end.line
            || (pos.line == range.end.line && pos.character <= range.end.character))
}
