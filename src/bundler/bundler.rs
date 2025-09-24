use std::collections::HashMap;

use swc_bundler::{Config, Hook, ModuleType};
use swc_common::{sync::Lrc, FileName, Globals, Mark, SourceMap, GLOBALS};
use swc_ecma_codegen::Node;
use swc_ecma_minifier::{optimize, option::MinifyOptions};
use swc_ecma_transforms::resolver;
use swc_ecma_visit::VisitMutWith;

use crate::bundler::{loader::Loader, Resolver};

pub struct Bundler {}

impl Bundler {
    pub fn new() -> Self {
        Self {}
    }

    pub fn bundle_entry(&self, entry: &str, out: &str) -> anyhow::Result<()> {
        let globals = Globals::default();

        GLOBALS.set(&globals, || {
            let cm = Lrc::new(SourceMap::default());

            let mut bundler = swc_bundler::Bundler::new(
                &globals,
                cm.clone(),
                Loader {},
                Resolver::new(),
                Config {
                    require: true,
                    module: ModuleType::Es,
                    ..Default::default()
                },
                Box::new(NoopHook),
            );

            let mut entries = HashMap::new();
            entries.insert(String::from("main"), FileName::Real(entry.into()));

            let bundles = bundler.bundle(entries)?;
            let top_level_mark = Mark::fresh(Mark::root());
            let unresolved_mark = top_level_mark;

            for bundle in bundles {
                let top_level_mark = Mark::fresh(Mark::root());

                let mut module = bundle.module;
                module.visit_mut_with(&mut resolver(unresolved_mark, top_level_mark, false));

                let minified = optimize(
                    module.into(),
                    cm.clone(),
                    None, // comments
                    None,
                    &MinifyOptions {
                        compress: Some(swc_ecma_minifier::option::CompressOptions {
                            bools: false,
                            conditionals: false,
                            sequences: 0,
                            ..Default::default()
                        }),
                        mangle: Some(Default::default()),
                        ..Default::default()
                    },
                    &swc_ecma_minifier::option::ExtraOptions {
                        top_level_mark,
                        unresolved_mark,
                        mangle_name_cache: None,
                    },
                );

                let mut buf = vec![];
                let mut cfg = swc_ecma_codegen::Config::default();
                cfg.minify = true;
                let mut emitter = swc_ecma_codegen::Emitter {
                    cfg,
                    cm: cm.clone(),
                    comments: None,
                    wr: swc_ecma_codegen::text_writer::JsWriter::new(
                        cm.clone(),
                        "\n",
                        &mut buf,
                        None,
                    ),
                };
                minified.emit_with(&mut emitter).unwrap();
                std::fs::write(out, buf)?;
            }

            Ok(())
        })
    }
}

struct NoopHook;

impl Hook for NoopHook {
    fn get_import_meta_props(
        &self,
        _span: swc_common::Span,
        _module_record: &swc_bundler::ModuleRecord,
    ) -> anyhow::Result<Vec<swc_ecma_ast::KeyValueProp>> {
        Ok(vec![])
    }
}
