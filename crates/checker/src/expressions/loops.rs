use tine_ast as ast;
use tine_common::{diagnostics::DiagnosticKind, locations::Locatable};
use tine_ir::{self as ir, Typed};
use tine_types::{store::TypeStore, types};

use super::TypeChecker;

impl TypeChecker {
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
        let Some((pattern, iterable, _)) = (|| {
            let (iterable, element_type) = self.visit_for_in_iterable(*node.iterable?);
            let iterable = iterable?;
            let pattern = self.visit_pattern(node.pattern?, &iterable, true, false)?;
            Some((pattern, iterable, element_type))
        })() else {
            node.body.map(|b| self.visit_block_expression(b));
            return None;
        };

        self.with_scope(|self_| {
            let body = node.body.map(|b| self_.visit_block_expression(b))?;

            Some(ir::ForInExpression {
                loc: node.loc,
                element: pattern,
                iterable: Box::new(iterable),
                ty: self_.get_loop_type(&body),
                body,
            })
        })
    }

    fn visit_for_in_iterable(
        &mut self,
        iterable: ast::Expression,
    ) -> (Option<ir::Expression>, types::TypeId) {
        let Some(iterable) = self.visit_expression(iterable) else {
            return (None, TypeStore::UNKNOWN);
        };
        let ty = match self.resolve(iterable.ty()) {
            types::Type::Ref(r) if r.inner == TypeStore::ARRAY => r.args[0],
            _ => {
                let error = DiagnosticKind::NotIterable {
                    type_name: self.types.display(iterable.ty()),
                };
                self.error(error, iterable.loc());
                TypeStore::UNKNOWN
            }
        };
        (Some(iterable), ty)
    }

    fn get_loop_type(&mut self, body: &ir::Block) -> types::TypeId {
        let breaks = body.find_breaks();
        if breaks.len() == 0 {
            return TypeStore::UNIT;
        }
        let first = breaks.first().unwrap();
        let ty = self.break_type(first);

        for stmt in breaks.iter().skip(1) {
            let curr = self.break_type(stmt);
            let got_immutable = match &stmt.expression {
                Some(expr) => self.is_mutable(expr) == Some(false),
                None => false,
            };
            self.check_assigned_type(ty, curr, got_immutable, stmt.loc);
        }

        self.intern(types::OptionType { some: ty })
    }

    fn break_type(&mut self, stmt: &ir::BreakStatement) -> types::TypeId {
        stmt.expression.as_ref().map_or(TypeStore::UNIT, |e| e.ty())
    }
}
