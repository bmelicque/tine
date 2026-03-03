#[cfg(test)]
use crate::analyzer::loader::ModuleLoader;

#[cfg(test)]
pub struct MockLoader;

#[cfg(test)]
impl ModuleLoader for MockLoader {
    fn load(&self, _: &crate::ModulePath) -> anyhow::Result<String> {
        Ok("".to_string())
    }
}
