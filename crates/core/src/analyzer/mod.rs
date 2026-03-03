mod builtins;
mod graph;
pub mod loader;
mod modules;
mod parse;
pub mod session;
mod std_modules;
mod type_check;

pub use self::loader::ModuleLoader;
pub use crate::analyzer::session::{Session, SessionLoader};
pub use modules::{Module, ModuleId, ModulePath, Source};
pub use type_check::ModuleTypeData;

pub fn analyze<'sess>(entry: ModulePath, loader: Box<SessionLoader>) -> Session {
    let mut session = Session::new(loader);
    let _ = session.analyze(entry);
    session
}
