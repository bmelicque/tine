use crate::{ast, codegen::CodeGenerator};

use swc_ecma_ast as swc;

impl CodeGenerator {
    pub fn item_to_swc(&mut self, node: &ast::Item) -> Vec<swc::ModuleItem> {
        match node {
            ast::Item::Statement(s) => self.stmt_to_swc(s).into_iter().map(|s| s.into()).collect(),
            ast::Item::UseDeclaration(u) => self.use_decl_to_swc(u).into(),
        }
    }

    fn use_decl_to_swc(&mut self, _node: &ast::UseDeclaration) -> Vec<swc::ModuleItem> {
        // TODO:
        Vec::new()
    }
}
