use std::path::PathBuf;

use crate::{
    analyzer::analyze,
    bundler::{
        bundler::bundle_entry, loader::SwcLoader, resolver::SwcResolver, utils::print_errors,
    },
};

pub fn transpile(entry_point: PathBuf, out: &str) {
    let Ok(result) = analyze(entry_point.clone()) else {
        return;
    };
    if print_errors(&result.modules) {
        return;
    }

    let resolver = SwcResolver::new();
    let loader = SwcLoader::new(result.modules);
    let _ = bundle_entry(entry_point, out, loader, resolver);
}
