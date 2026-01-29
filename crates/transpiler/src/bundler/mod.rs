mod bundler;
mod internals;
mod loader;
mod resolver;
mod transpiler;

use loader::SwcLoader;
use resolver::SwcResolver;
pub use transpiler::transpile;
