use crate::bundler::{bundler::bundle_entry, loader::SwcLoader, resolver::SwcResolver};
use mylang_core::{analyze, pretty_print_error};
use std::path::PathBuf;

pub fn transpile(entry_point: PathBuf, out: &str) {
    let session = analyze(entry_point.clone());

    let mut has_errors = false;
    for (&module_id, diagnostics) in session.diagnostics() {
        let src = &session.read_module(module_id).src;
        for diag in diagnostics {
            has_errors = true;
            pretty_print_error(src, diag);
        }
    }
    if has_errors {
        return;
    }

    let resolver = SwcResolver::new();
    let loader = SwcLoader::new(&session);
    let _ = bundle_entry(entry_point, out, loader, resolver);
}
