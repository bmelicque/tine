use crate::{
    codegen::{utils::ident_from_str, CodeGenerator},
    utils::{make_relative, modulepath_to_filename},
};

use tine_core::{ir, ModulePath};

use swc_common::{FileName, DUMMY_SP};
use swc_ecma_ast as swc;

impl CodeGenerator<'_> {
    pub fn item_to_swc(&mut self, node: &ir::Statement) -> Vec<swc::ModuleItem> {
        match node {
            ir::Statement::Use(u) => vec![self.use_decl_to_swc(u).into()],
            stmt => self.stmt_to_swc(stmt).into_iter().map(Into::into).collect(),
        }
    }

    fn use_decl_to_swc(&mut self, node: &ir::UseDeclaration) -> swc::ModuleItem {
        let module_name = modulepath_to_filename(&node.path);
        let src = self.get_imports_src(module_name);
        let specifiers = node
            .symbols
            .iter()
            .map(|s| self.specifier_to_swc(&s.as_name()))
            .collect();

        swc::ModuleItem::ModuleDecl(swc::ModuleDecl::Import(swc::ImportDecl {
            span: DUMMY_SP,
            specifiers,
            src,
            type_only: false,
            with: None,
            phase: swc::ImportPhase::Evaluation,
        }))
    }

    fn get_imports_src(&self, name: FileName) -> Box<swc::Str> {
        match name {
            FileName::Real(filename) => {
                let ModulePath::Real(current) = self.get_filename() else {
                    panic!("unexpected filename variant")
                };
                let relative = make_relative(current, &filename);
                Box::new(relative.to_str().unwrap().into())
            }
            FileName::Custom(filename) => Box::new(filename.into()),
            _ => unreachable!(),
        }
    }

    fn specifier_to_swc(&self, name: &str) -> swc::ImportSpecifier {
        let id = ident_from_str(name);
        swc::ImportSpecifier::Named(swc::ImportNamedSpecifier {
            span: DUMMY_SP,
            local: id.clone(),
            imported: Some(swc::ModuleExportName::Ident(id)),
            is_type_only: false,
        })
    }
}
