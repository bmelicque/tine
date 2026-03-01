mod bundler;
mod codegen;
mod utils;

use std::path::PathBuf;

use tine_core::SessionLoader;

pub fn transpile(entry_point: &PathBuf, loader: Box<SessionLoader>) -> anyhow::Result<String> {
    let filename = tine_core::ModulePath::Real(entry_point.canonicalize().unwrap());
    bundler::transpile(&filename, loader)
}
