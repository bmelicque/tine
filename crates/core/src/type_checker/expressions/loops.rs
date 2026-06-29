use crate::{
    ast,
    diagnostics::DiagnosticKind,
    ir,
    type_checker::{analysis_context::type_store::TypeStore, patterns::lower_pattern},
    types::{self, OptionType, TypeId},
    Location,
};

use super::TypeChecker;

impl TypeChecker<'_> {
    pub fn visit_loop(&mut self, node: ast::Loop) -> Option<ir::Expression> {
        match node {
            ast::Loop::For(node) => self.visit_for_expression(node).map(Into::into),
            ast::Loop::ForIn(node) => self.visit_for_in_expression(node).map(Into::into),
        }
    }

    fn visit_for_expression(&mut self, node: ast::ForExpression) -> Option<ir::ForExpression> {
        let body = node.body.map(|b| self.visit_block_expression(b));

        let condition = match node.condition {
            Some(c) => Some(Box::new(self.visit_condition(*c)?)),
            None => None,
        };
        let body = body?;
        let ty = self.get_loop_type(&body);

        Some(ir::ForExpression {
            loc: node.loc,
            condition,
            body,
            ty,
        })
    }

    fn visit_for_in_expression(
        &mut self,
        node: ast::ForInExpression,
    ) -> Option<ir::ForInExpression> {
        let pattern_loc = node
            .pattern
            .as_ref()
            .map_or(Location::default(), |p| p.loc());
        let Some((pattern, iterable, element)) = (|| {
            let (iterable, element_type) = self.visit_for_in_iterable(*node.iterable?);
            let iterable = iterable?;
            let pattern = self.visit_pattern(node.pattern?, &iterable, true)?;
            Some((pattern, iterable, element_type))
        })() else {
            node.body.map(|b| self.visit_block_expression(b));
            return None;
        };

        let lowered = lower_pattern(pattern, iterable.clone());
        self.with_scope(|self_| {
            let element = self_.make_temp_variable_with_type(pattern_loc, &iterable, element);
            let mut body = node.body.map(|b| self_.visit_block_expression(b))?;
            body.statements.splice(
                0..0,
                lowered
                    .decls
                    .into_iter()
                    .map(Into::into)
                    .collect::<Vec<_>>(),
            );

            let guard = self_.make_guard(lowered.test, element.loc)?;
            body.statements.insert(0, guard);

            Some(ir::ForInExpression {
                loc: node.loc,
                element,
                iterable: Box::new(iterable),
                ty: self_.get_loop_type(&body),
                body,
            })
        })
    }

    fn make_guard(&mut self, test: Option<ir::Expression>, loc: Location) -> Option<ir::Statement> {
        let Some(test) = test else {
            self.error(DiagnosticKind::RefutablePatternExpected, loc);
            return None;
        };

        Some(ir::Statement::Expression(ir::Expression::If(
            ir::IfExpression {
                loc,
                consequent: ir::Block::from(ir::Statement::Continue(ir::ContinueStatement { loc })),
                condition: Box::new(ir::Expression::Unary(ir::UnaryExpression {
                    loc,
                    operator: ir::UnaryOperator::Bang,
                    operand: Box::new(test),
                    ty: TypeStore::BOOLEAN,
                })),
                alternate: None,
                ty: TypeStore::UNIT,
            },
        )))
    }

    fn visit_for_in_iterable(
        &mut self,
        iterable: ast::Expression,
    ) -> (Option<ir::Expression>, TypeId) {
        let Some(iterable) = self.visit_expression(iterable) else {
            return (None, TypeStore::UNKNOWN);
        };
        let ty = match self.resolve(iterable.ty()) {
            types::Type::Array(a) => a.element,
            _ => {
                let error = DiagnosticKind::NotIterable {
                    type_name: self.session.display_type(iterable.ty()),
                };
                self.error(error, iterable.loc());
                TypeStore::UNKNOWN
            }
        };
        (Some(iterable), ty)
    }

    fn get_loop_type(&mut self, body: &ir::Block) -> TypeId {
        let breaks = body.find_breaks();
        if breaks.len() == 0 {
            return TypeStore::UNIT;
        }
        let first = breaks.first().unwrap();
        let ty = self.break_type(first);

        for stmt in breaks.iter().skip(1) {
            let curr = self.break_type(stmt);
            let got_immutable = match &stmt.expression {
                Some(expr) => expr.is_mutable() == Some(false),
                None => false,
            };
            self.check_assigned_type(ty, curr, got_immutable, stmt.loc);
        }

        self.intern(OptionType { some: ty })
    }

    fn break_type(&mut self, stmt: &ir::BreakStatement) -> TypeId {
        stmt.expression.as_ref().map_or(TypeStore::UNIT, |e| e.ty())
    }
}
