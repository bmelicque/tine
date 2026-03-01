use anyhow::Result;
use std::{
    collections::HashMap,
    fs,
    sync::{Arc, RwLock},
};
use tine_core::{ModuleLoader, ModulePath};
use url::Url;

pub type OpenFiles = Arc<RwLock<HashMap<Url, String>>>;

pub struct LspLoader {
    open_files: OpenFiles,
}

impl LspLoader {
    pub fn new(open_files: OpenFiles) -> Self {
        Self { open_files }
    }
}

impl ModuleLoader for LspLoader {
    fn load(&self, path: &ModulePath) -> Result<String> {
        match path {
            ModulePath::Real(p) => {
                let url = url::Url::from_file_path(p)
                    .map_err(|_| anyhow::anyhow!("invalid file path"))?;

                match self.open_files.read().unwrap().get(&url) {
                    Some(src) => Ok(src.clone()),
                    None => Ok(fs::read_to_string(p)?),
                }
            }
            ModulePath::Virtual(_) => Ok("".to_string()),
        }
    }
}
