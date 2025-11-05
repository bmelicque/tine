use std::path::{Path, PathBuf};

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
