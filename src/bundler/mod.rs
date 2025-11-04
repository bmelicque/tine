mod bundler;
mod internals;
mod loader;
mod resolver;
mod transpiler;
mod utils;

use loader::SwcLoader;
use resolver::SwcResolver;
pub use transpiler::transpile;
