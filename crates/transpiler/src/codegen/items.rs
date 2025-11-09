use crate::{
    codegen::{utils::create_ident, CodeGenerator},
    utils::make_relative,
};

use mylang_core::{ast, use_decl_to_paths, ModuleImports};

use swc_common::{FileName, DUMMY_SP};
use swc_ecma_ast as swc;

impl CodeGenerator {
    pub fn item_to_swc(&mut self, node: &ast::Item) -> Vec<swc::ModuleItem> {
        match node {
            ast::Item::Invalid(_) => {
                unreachable!("Invalid input should've been detected during analysis phase")
            }
            ast::Item::Statement(s) => self.stmt_to_swc(s).into_iter().map(|s| s.into()).collect(),
            ast::Item::UseDeclaration(u) => self.use_decl_to_swc(u).into(),
        }
    }

    fn use_decl_to_swc(&mut self, node: &ast::UseDeclaration) -> Vec<swc::ModuleItem> {
        use_decl_to_paths(self.get_filename(), node)
            .into_iter()
            .map(|imports| self.imports_to_swc(imports))
            .collect()
    }

    fn imports_to_swc(&mut self, imports: ModuleImports) -> swc::ModuleItem {
        let src = self.get_imports_src(imports.module_name);
        let specifiers: Vec<_> = imports
            .import_tree
            .into_iter()
            .map(|tree| self.specifier_to_swc(tree))
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
                let FileName::Real(current) = self.get_filename() else {
                    panic!("unexpected filename variant")
                };
                let relative = make_relative(current, &filename);
                Box::new(relative.to_str().unwrap().into())
            }
            FileName::Custom(filename) => Box::new(filename.into()),
            _ => unreachable!(),
        }
    }

    fn specifier_to_swc(&self, tree: ast::UseTree) -> swc::ImportSpecifier {
        let id = create_ident(tree.path[0].as_str());
        swc::ImportSpecifier::Named(swc::ImportNamedSpecifier {
            span: DUMMY_SP,
            local: id.clone(),
            imported: Some(swc::ModuleExportName::Ident(id)),
            is_type_only: false,
        })
    }
}
