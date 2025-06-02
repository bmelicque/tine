use crate::{ast, parser::parser::ParseError, types::Type};

use super::TypeChecker;

impl TypeChecker {
    pub fn visit_loop(&mut self, node: &ast::Loop) -> Type {
        match node {
            ast::Loop::For(node) => self.visit_for_expression(node),
            ast::Loop::ForIn(node) => self.visit_for_in_expression(node),
        }
    }

    fn visit_for_expression(&mut self, node: &ast::ForExpression) -> Type {
        self.visit_condition(&node.condition);
        self.symbols.enter_scope();
        self.visit_block_expression(&node.body);
        self.symbols.exit_scope();
        // FIXME: break type
        Type::Unit
    }

    fn visit_for_in_expression(&mut self, node: &ast::ForInExpression) -> Type {
        let inferred_type = match self.visit_expression(&node.iterable) {
            Type::Array(ty) => *ty.clone(),
            ty => {
                self.errors.push(ParseError {
                    message: format!("Type {} is not iterable", ty),
                    span: node.iterable.as_span(),
                });
                Type::Unknown
            }
        };
        self.symbols.enter_scope();
        let mut variables = Vec::<(String, Type)>::new();
        self.match_pattern(&node.pattern, inferred_type, &mut variables);
        for (name, ty) in variables {
            self.symbols.define(&name, ty, false);
        }
        self.visit_block_expression(&node.body);
        self.symbols.exit_scope();
        // FIXME: break type
        Type::Unit
    }
}
