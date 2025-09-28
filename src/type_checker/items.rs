use crate::{ast, type_checker::TypeChecker};

impl TypeChecker {
    pub fn visit_item(&mut self, node: &ast::Item) {
        match node {
            ast::Item::Statement(s) => {
                self.visit_statement(s);
            }
            ast::Item::UseDeclaration(u) => self.visit_use_declaration(u),
        }
    }

    fn visit_use_declaration(&mut self, _node: &ast::UseDeclaration) {
        // TODO: type-check file...
        // FIXME:
    }
}
