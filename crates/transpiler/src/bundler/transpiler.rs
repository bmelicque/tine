use crate::bundler::{bundler::bundle_entry, loader::SwcLoader, resolver::SwcResolver};
use tine_core::{analyze, pretty_print_error, ModulePath, SessionLoader};

pub fn transpile(entry_point: &ModulePath, loader: Box<SessionLoader>) -> anyhow::Result<String> {
    let session = analyze(entry_point.clone(), loader);

    let mut has_errors = false;
    for (&module_id, diagnostics) in session.diagnostics() {
        let src = &session.read_module(module_id).src;
        for diag in diagnostics {
            has_errors = true;
            pretty_print_error(src, diag);
        }
    }
    if has_errors {
        anyhow::bail!("");
    }

    let resolver = SwcResolver::new();
    let loader = SwcLoader::new(&session);
    bundle_entry(entry_point, loader, resolver)
}
