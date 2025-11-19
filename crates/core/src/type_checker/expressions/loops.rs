use crate::{
    ast,
    type_checker::{
        analysis_context::{type_store::TypeStore, SymbolData, SymbolRef},
        SymbolKind,
    },
    types::{self, OptionType, Type, TypeId},
};

use super::TypeChecker;

impl TypeChecker {
    pub fn visit_loop(&mut self, node: &ast::Loop) -> TypeId {
        match node {
            ast::Loop::For(node) => self.visit_for_expression(node),
            ast::Loop::ForIn(node) => self.visit_for_in_expression(node),
        }
    }

    fn visit_for_expression(&mut self, node: &ast::ForExpression) -> TypeId {
        self.visit_condition(&node.condition);
        let ty = self.with_scope(node.span, |checker| checker.visit_loop_body(&node.body));

        self.analysis_context.save_expression_type(node.span, ty)
    }

    fn visit_for_in_expression(&mut self, node: &ast::ForInExpression) -> TypeId {
        let (inferred_type, dependencies) = self.visit_for_in_iterable(&node.iterable);
        let ty = self.with_scope(node.span, |checker| {
            let mut variables = vec![];
            checker.match_pattern(&node.pattern, inferred_type.clone(), &mut variables);
            for (name, ty) in variables {
                checker.analysis_context.register_symbol(SymbolData::new(
                    name.clone(),
                    SymbolKind::Value,
                    ty,
                    false,
                    node.pattern.as_span(),
                    dependencies.clone(),
                ));
            }
            checker.visit_loop_body(&node.body)
        });

        self.analysis_context.save_expression_type(node.span, ty)
    }

    fn visit_for_in_iterable(&mut self, iterable: &ast::Expression) -> (TypeId, Vec<SymbolRef>) {
        self.with_dependencies(|checker| {
            let ty = checker.visit_expression(iterable);
            match checker.resolve(ty) {
                types::Type::Array(ty) => ty.element,
                ty => {
                    checker.error(format!("Type {} is not iterable", ty), iterable.as_span());
                    TypeStore::UNKNOWN
                }
            }
        })
    }

    fn visit_loop_body(&mut self, node: &ast::BlockExpression) -> TypeId {
        self.visit_block_expression(node);
        let mut breaks = Vec::<ast::BreakStatement>::new();
        node.find_breaks(&mut breaks);
        if breaks.len() == 0 {
            return TypeStore::UNIT;
        }

        let first = breaks.first().unwrap();
        let ty = self.break_type(first);

        for stmt in breaks.iter().skip(1) {
            let curr = self.break_type(stmt);
            self.check_assigned_type(ty, curr, stmt.span);
        }

        self.analysis_context
            .type_store
            .add(Type::Option(OptionType { some: ty }))
    }

    fn break_type(&mut self, stmt: &ast::BreakStatement) -> TypeId {
        stmt.value
            .as_ref()
            .map(|expr| self.get_type_at(expr.as_span()).unwrap())
            .unwrap_or(TypeStore::UNIT)
    }
}
