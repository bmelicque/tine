use crate::{
    ast,
    diagnostics::DiagnosticKind,
    type_checker::{
        analysis_context::{type_store::TypeStore, SymbolData, SymbolRef},
        patterns::TokenList,
    },
    types::{self, OptionType, Type, TypeId},
    SymbolKind,
};

use super::TypeChecker;

impl TypeChecker<'_> {
    pub fn visit_loop(&mut self, node: &ast::Loop) -> TypeId {
        match node {
            ast::Loop::For(node) => self.visit_for_expression(node),
            ast::Loop::ForIn(node) => self.visit_for_in_expression(node),
        }
    }

    fn visit_for_expression(&mut self, node: &ast::ForExpression) -> TypeId {
        self.visit_condition(&node.condition);
        let ty = self.with_scope(|checker| checker.visit_loop_body(&node.body));

        self.ctx.save_expression_type(node.loc, ty)
    }

    fn visit_for_in_expression(&mut self, node: &ast::ForInExpression) -> TypeId {
        let (inferred_type, dependencies) = self.visit_for_in_iterable(&node.iterable);
        let ty = self.with_scope(|checker| {
            let mut variables = TokenList::new();
            checker.match_pattern(&node.pattern, inferred_type.clone(), &mut variables);
            for (name, ty) in variables.0 {
                checker.ctx.register_symbol(SymbolData {
                    name: name.as_str().into(),
                    ty,
                    kind: SymbolKind::constant(),
                    defined_at: node.pattern.loc(),
                    dependencies: dependencies.clone(),
                    ..Default::default()
                });
            }
            checker.visit_loop_body(&node.body)
        });

        self.ctx.save_expression_type(node.loc, ty)
    }

    fn visit_for_in_iterable(&mut self, iterable: &ast::Expression) -> (TypeId, Vec<SymbolRef>) {
        self.with_dependencies(|checker| {
            let ty = checker.visit_expression(iterable);
            match checker.resolve(ty) {
                types::Type::Array(ty) => ty.element,
                _ => {
                    let error = DiagnosticKind::NotIterable {
                        type_name: self.session.display_type(ty),
                    };
                    checker.error(error, iterable.loc());
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
            self.check_assigned_type(ty, curr, stmt.loc);
        }

        self.intern(Type::Option(OptionType { some: ty }))
    }

    fn break_type(&mut self, stmt: &ast::BreakStatement) -> TypeId {
        stmt.value
            .as_ref()
            .map(|expr| self.get_type_at(expr.loc()).unwrap())
            .unwrap_or(TypeStore::UNIT)
    }
}
