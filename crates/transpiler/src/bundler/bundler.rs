use std::collections::HashMap;

use swc_bundler::{Config, Hook, ModuleType};
use swc_common::{sync::Lrc, FileName, Globals, Mark, SourceMap, GLOBALS};
use swc_ecma_codegen::Node;
use swc_ecma_minifier::{optimize, option::MinifyOptions};
use swc_ecma_transforms::resolver;
use swc_ecma_visit::VisitMutWith;
use tine_core::ModulePath;

use crate::bundler::{SwcLoader, SwcResolver};

pub fn bundle_entry(
    filename: &ModulePath,
    loader: SwcLoader,
    swc_resolver: SwcResolver,
) -> anyhow::Result<String> {
    let ModulePath::Real(filename) = filename else {
        panic!()
    };
    let globals = Globals::default();

    GLOBALS.set(&globals, || {
        let cm = Lrc::new(SourceMap::default());

        let mut bundler = swc_bundler::Bundler::new(
            &globals,
            cm.clone(),
            loader,
            swc_resolver,
            Config {
                require: true,
                module: ModuleType::Es,
                ..Default::default()
            },
            Box::new(NoopHook),
        );

        let mut entries = HashMap::new();
        entries.insert(String::from("main"), FileName::Real(filename.clone()));

        let bundles = bundler.bundle(entries)?;
        let top_level_mark = Mark::fresh(Mark::root());
        let unresolved_mark = top_level_mark;

        let mut output = String::new();
        for bundle in bundles {
            let top_level_mark = Mark::fresh(Mark::root());

            let mut module = bundle.module;
            module.visit_mut_with(&mut resolver(unresolved_mark, top_level_mark, false));

            let minified = optimize(
                module.into(),
                cm.clone(),
                None,
                None,
                &MinifyOptions {
                    compress: Some(swc_ecma_minifier::option::CompressOptions {
                        bools: false,
                        conditionals: false,
                        sequences: 0,
                        ..Default::default()
                    }),
                    mangle: None,
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
                wr: swc_ecma_codegen::text_writer::JsWriter::new(cm.clone(), "\n", &mut buf, None),
            };
            minified.emit_with(&mut emitter).unwrap();
            output.push_str(&String::from_utf8(buf)?);
        }

        Ok(output)
    })
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
