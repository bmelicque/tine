use swc_common::FileName;
use swc_ecma_loader::resolve::Resolution;

pub struct Resolver {}

impl Resolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl swc_bundler::Resolve for Resolver {
    fn resolve(
        &self,
        base: &FileName,
        module_specifier: &str,
    ) -> Result<Resolution, anyhow::Error> {
        // virtual module
        if module_specifier == "internals" {
            return Ok(Resolution {
                filename: FileName::Custom("internals".into()),
                slug: None,
            });
        }

        // simple relative path resolution
        let base_dir = match base {
            FileName::Real(p) => p.parent().unwrap_or_else(|| std::path::Path::new("")),
            _ => std::path::Path::new(""),
        };
        let resolved_path = base_dir.join(module_specifier);
        Ok(Resolution {
            filename: FileName::Real(resolved_path),
            slug: None,
        })
    }
}
