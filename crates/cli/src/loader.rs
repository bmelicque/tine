use tine_core::{ModuleLoader, ModulePath};

pub struct CliLoader;

impl ModuleLoader for CliLoader {
    fn load<'a>(&'a self, path: &ModulePath) -> anyhow::Result<String> {
        match path {
            ModulePath::Real(buf) => std::fs::read_to_string(buf).map_err(|e| anyhow::anyhow!(e)),
            ModulePath::Virtual(_) => Ok("".to_string()),
        }
    }
}
