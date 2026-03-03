use crate::ModulePath;

pub trait ModuleLoader {
    fn load(&self, path: &ModulePath) -> anyhow::Result<String>;
}
