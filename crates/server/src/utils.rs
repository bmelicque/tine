use url::Url;

pub fn normalize_file_url(url: &Url) -> Option<Url> {
    if url.scheme() != "file" {
        return Some(url.clone());
    }

    Url::from_file_path(std::fs::canonicalize(url.to_file_path().unwrap()).unwrap()).ok()
}
