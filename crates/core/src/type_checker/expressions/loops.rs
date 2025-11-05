use crate::{ast, parser::parser::ParseError, type_checker::analysis_context::Symbol, types};

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
        let ty = self.with_scope(node.span, |checker| checker.visit_loop_body(&node.body));
        self.set_type_at(node.span, ty)
    }

    fn visit_for_in_expression(&mut self, node: &ast::ForInExpression) -> types::Type {
        let (inferred_type, dependencies) = self.visit_for_in_iteratable(&node.iterable);
        let ty = self.with_scope(node.span, |checker| {
            let mut variables = Vec::<(String, types::Type)>::new();
            checker.match_pattern(&node.pattern, inferred_type.clone(), &mut variables);
            for (name, ty) in variables {
                checker.analysis_context.register_symbol(Symbol::new(
                    name.clone(),
                    ty.clone(),
                    false,
                    node.pattern.as_span(),
                    dependencies.clone(),
                ));
            }
            checker.visit_loop_body(&node.body)
        });

        self.set_type_at(node.span, ty)
    }

    fn visit_for_in_iteratable(&mut self, iterable: &ast::Expression) -> (types::Type, Vec<usize>) {
        self.with_dependencies(|checker| match checker.visit_expression(iterable) {
            types::Type::Array(ty) => *ty.element.clone(),
            ty => {
                checker.errors.push(ParseError {
                    message: format!("Type {} is not iterable", ty),
                    span: iterable.as_span(),
                });
                types::Type::Unknown
            }
        })
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
            if !self.can_be_assigned_to(&curr, &ty) {
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
            .map(|expr| self.get_type_at(expr.as_span()).unwrap())
            .unwrap_or(types::Type::Unit)
    }
}
