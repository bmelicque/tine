mod bundler;
mod codegen;
mod ownership_analyser;
mod utils;

use std::path::PathBuf;

pub use crate::bundler::SwcLoader;
use crate::bundler::{bundle_entry, SwcResolver};

pub fn transpile(entry_point: &PathBuf, loader: SwcLoader) -> anyhow::Result<String> {
    let filename = tine_common::module_path::ModulePath::Real(entry_point.canonicalize().unwrap());
    let resolver = SwcResolver::new();
    bundle_entry(&filename, loader, resolver)
}
