mod bundler;
mod codegen;
mod ownership_analyser;
mod utils;

use std::path::PathBuf;

pub use crate::bundler::SwcLoader;
use crate::bundler::{bundle_entry, BundleOptions, SwcResolver};

#[derive(Default)]
pub struct TranspileOptions {
    pub minify: bool,
}
impl From<TranspileOptions> for BundleOptions {
    fn from(value: TranspileOptions) -> Self {
        Self {
            minify: value.minify,
        }
    }
}

pub fn transpile(
    entry_point: &PathBuf,
    loader: SwcLoader,
    options: TranspileOptions,
) -> anyhow::Result<String> {
    let filename = tine_common::module_path::ModulePath::Real(entry_point.canonicalize().unwrap());
    let resolver = SwcResolver::new();
    bundle_entry(&filename, loader, resolver, options.into())
}
