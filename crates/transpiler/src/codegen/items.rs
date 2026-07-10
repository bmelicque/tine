use crate::{
    codegen::{utils::ident_from_str, CodeGenerator},
    utils::{make_relative, modulepath_to_filename},
};

use swc_common::{FileName, DUMMY_SP};
use swc_ecma_ast as swc;
use tine_common::module_path::ModulePath;
use tine_ir as ir;

impl CodeGenerator<'_, '_> {
    pub fn item_to_swc(&mut self, node: ir::Statement) -> Vec<swc::ModuleItem> {
        match node {
            ir::Statement::Enum(e) => vec![self.enum_def_to_swc(e).into()],
            ir::Statement::Function(f) => {
                vec![self.handle_top_level_function(f)]
            }
            ir::Statement::Struct(s) => vec![self.struct_def_to_swc(s).into()],
            ir::Statement::Use(u) => vec![self.use_decl_to_swc(u).into()],
            ir::Statement::Variable(node) => self.handle_top_level_declaration(node),
            stmt => self.stmt_to_swc(stmt).into_iter().map(Into::into).collect(),
        }
    }

    fn use_decl_to_swc(&mut self, node: ir::UseDeclaration) -> swc::ModuleItem {
        let module_name = modulepath_to_filename(&node.path);
        let src = self.get_imports_src(module_name);
        let specifiers = node
            .symbols
            .iter()
            .map(|s| {
                let name = self.symbols.get_symbol(*s).name();
                self.specifier_to_swc(name)
            })
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
                let ModulePath::Real(current) = &self.name else {
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

    fn handle_top_level_function(&mut self, node: ir::FunctionDefinition) -> swc::ModuleItem {
        match node.name.1 {
            ir::FunctionName::Function(f) => {
                let name = &self.symbols.get(f).name;
                swc::ModuleItem::from(swc::FnDecl {
                    ident: ident_from_str(name),
                    declare: false,
                    function: Box::new(self.handle_function(node.params, node.body)),
                })
            }
            ir::FunctionName::StaticMethod(m) => self.handle_static_method(node, m).into(),
        }
    }

    fn handle_top_level_declaration(
        &mut self,
        node: ir::VariableDeclaration,
    ) -> Vec<swc::ModuleItem> {
        let (stmts, decl) = self.declaration_helper(node);
        let mut items = stmts.into_iter().map(Into::into).collect::<Vec<_>>();
        items.push(swc::ModuleItem::ModuleDecl(swc::ModuleDecl::ExportDecl(
            swc::ExportDecl {
                span: DUMMY_SP,
                decl,
            },
        )));
        items
    }
}
