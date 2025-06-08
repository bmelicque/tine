use crate::{ast, parser::parser::ParseError, types};

use super::TypeChecker;

impl TypeChecker {
    pub fn visit_loop(&mut self, node: &ast::Loop) -> types::Type {
        match node {
            ast::Loop::For(node) => self.visit_for_expression(node),
            ast::Loop::ForIn(node) => self.visit_for_in_expression(node),
        }
    }

    fn visit_for_expression(&mut self, node: &ast::ForExpression) -> types::Type {
        self.visit_condition(&node.condition);
        self.symbols.enter_scope();
        let ty = self.visit_loop_body(&node.body);
        self.symbols.exit_scope();
        ty
    }

    fn visit_for_in_expression(&mut self, node: &ast::ForInExpression) -> types::Type {
        let inferred_type = match self.visit_expression(&node.iterable) {
            types::Type::Array(ty) => *ty.element.clone(),
            ty => {
                self.errors.push(ParseError {
                    message: format!("Type {} is not iterable", ty),
                    span: node.iterable.as_span(),
                });
                types::Type::Unknown
            }
        };
        self.symbols.enter_scope();
        let mut variables = Vec::<(String, types::Type)>::new();
        self.match_pattern(&node.pattern, inferred_type, &mut variables);
        for (name, ty) in variables {
            self.symbols.define(&name, ty, false);
        }
        let ty = self.visit_loop_body(&node.body);
        self.symbols.exit_scope();
        ty
    }

    fn visit_loop_body(&mut self, node: &ast::BlockExpression) -> types::Type {
        self.visit_block_expression(node);
        let mut breaks = Vec::<ast::BreakStatement>::new();
        node.find_breaks(&mut breaks);
        if breaks.len() == 0 {
            return types::Type::Unit;
        }

        let first = breaks.first().unwrap();
        let ty = self.break_type(first);

        for stmt in breaks.iter().skip(1) {
            let curr = self.break_type(stmt);
            if !curr.is_assignable_to(&ty) {
                self.errors.push(ParseError {
                    message: format!("Type {} doesn't match type {}", curr, ty),
                    span: stmt.span,
                });
            }
        }

        types::OptionType { some: Box::new(ty) }.into()
    }

    fn break_type(&mut self, stmt: &ast::BreakStatement) -> types::Type {
        stmt.value
            .as_ref()
            // FIXME: this report possible errors a second time!
            .map(|expr| self.visit_expression(&expr))
            .unwrap_or(types::Type::Unit)
    }
}
