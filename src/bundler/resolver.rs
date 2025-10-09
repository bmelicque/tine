use swc_common::FileName;
use swc_ecma_loader::resolve::Resolution;
pub struct SwcResolver {}

impl SwcResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl swc_bundler::Resolve for SwcResolver {
    fn resolve(
        &self,
        base: &FileName,
        module_specifier: &str,
    ) -> Result<Resolution, anyhow::Error> {
        let filename = if module_specifier.starts_with(".") {
            let base_dir = match base {
                FileName::Real(p) => p.parent().unwrap_or_else(|| std::path::Path::new("")),
                _ => std::path::Path::new(""),
            };
            FileName::Real(base_dir.join(module_specifier))
        } else {
            FileName::Custom(module_specifier.to_string())
        };

        Ok(Resolution {
            filename,
            slug: None,
        })
    }
}
