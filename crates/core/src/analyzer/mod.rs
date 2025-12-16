mod graph;
mod modules;
mod parse;
pub mod session;
mod type_check;

use std::path::PathBuf;

pub use modules::{Module, ModuleId, ModulePath, Source};
pub use type_check::ModuleTypeData;

use crate::analyzer::session::Session;

pub fn analyze(entry: PathBuf) -> Session {
    let mut session = Session::new();
    let _ = session.analyze(entry);
    session
}
