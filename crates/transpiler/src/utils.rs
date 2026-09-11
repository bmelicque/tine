use std::path::{Path, PathBuf};

use swc_common::FileName;
use tine_common::module_path::ModulePath;
use tine_ir as ir;
use tine_symbols::table::SymbolTable;

/// Compute a relative path from `base` to `path`.
/// Works even if `path` is outside of `base` (e.g. gives `../../other/file`).
pub fn make_relative(base: &Path, path: &Path) -> PathBuf {
    let base_directory = if base.is_file() {
        base.parent().unwrap()
    } else {
        base
    };
    let base = base_directory.components().collect::<Vec<_>>();
    let path = path.components().collect::<Vec<_>>();

    // Find common prefix length
    let common_prefix_len = base.iter().zip(&path).take_while(|(a, b)| a == b).count();

    // Steps to go up from base to common ancestor
    let mut rel = PathBuf::new();
    if base.len() == common_prefix_len {
        rel.push(".")
    } else {
        for _ in common_prefix_len..base.len() {
            rel.push("..");
        }
    }

    // Steps down to target
    for comp in path.iter().skip(common_prefix_len) {
        rel.push(comp.as_os_str());
    }

    rel
}

pub fn modulepath_to_filename(name: &ModulePath) -> FileName {
    match name {
        ModulePath::Real(path) => FileName::Real(path.clone()),
        ModulePath::Virtual(name) => FileName::Custom(name.clone()),
    }
}

pub fn is_declaration_mutable(node: &ir::VariableDeclaration, symbols: &SymbolTable) -> bool {
    node.pattern
        .walk()
        .filter_map(|n| n.as_pattern())
        .filter_map(|p| p.as_identifier())
        .any(|i| symbols.is_mutable(i.symbol))
}
