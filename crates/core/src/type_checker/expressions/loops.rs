use crate::{
    ast,
    diagnostics::DiagnosticKind,
    ir,
    type_checker::{
        analysis_context::{type_store::TypeStore, SymbolData},
        utils::{make_simple_declaration, make_tmp_identifier},
    },
    types::{self, OptionType, TypeId},
    SymbolKind,
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
        let node = self.lower_for_in_expression(node);

        let (iterable, iter_type) = match node.iterable {
            Some(i) => self.visit_for_in_iterable(*i),
            None => (None, TypeStore::UNKNOWN),
        };

        let (Some(pattern), Some(iterable)) = (node.pattern, iterable) else {
            self.visit_block_expression(node.body?);
            return None;
        };

        let ast::Pattern::Identifier(ast::IdentifierPattern(ident)) = pattern else {
            panic!("expected an identifier pattern (other cases should've been lowered)")
        };

        let body = node.body?;
        let (ident, body) = self.with_scope(|self_| {
            let ident = ir::Identifier {
                loc: ident.loc,
                symbol: self_.ctx.register_symbol(SymbolData {
                    name: ident.text,
                    ty: iter_type,
                    kind: SymbolKind::Value { mutable: false },
                    defined_at: ident.loc,
                    dependencies: iterable.dependencies().map(|d| d.symbol.clone()).collect(),
                    ..Default::default()
                }),
            };
            let body = self_.visit_block_expression(body);
            (ident, body)
        });

        let ty = self.get_loop_type(&body);

        Some(ir::ForInExpression {
            loc: node.loc,
            element: ident,
            iterable: Box::new(iterable),
            body,
            ty,
        })
    }

    /// Lower pattern matching in the loop.
    ///
    /// For example:
    ///
    /// ```tine
    /// for User {name, age} in users {
    ///     ...
    /// }
    ///
    /// // becomes
    /// for tmp in userResults {
    ///     const name = tmp.name
    ///     const age = tmp.age
    ///     ...
    /// }
    /// ```
    fn lower_for_in_expression(&mut self, node: ast::ForInExpression) -> ast::ForInExpression {
        match &node.pattern {
            Some(ast::Pattern::Identifier(_)) | None => return node,
            Some(_) => {}
        };
        let pattern = node.pattern.unwrap();
        let element = make_tmp_identifier(pattern.loc());

        let desugared = self.desugar_pattern(pattern, element.clone().into(), false);
        let guard = desugared.test.map(|test| {
            let loc = test.loc();
            ast::Statement::Expression(ast::ExpressionStatement {
                expression: Box::new(ast::Expression::If(ast::IfExpression {
                    loc,
                    condition: Some(Box::new(ast::Expression::Unary(ast::UnaryExpression {
                        loc,
                        operator: ast::UnaryOperator::Bang,
                        operand: Some(Box::new(test)),
                    }))),
                    consequent: Some(ast::BlockExpression {
                        loc,
                        statements: vec![ast::Statement::Continue(ast::ContinueStatement { loc })],
                    }),
                    alternate: None,
                })),
            })
        });
        let mut statements: Vec<ast::Statement> = desugared
            .bindings
            .into_iter()
            .map(|(identifier, against)| make_simple_declaration(identifier, against).into())
            .collect();
        if let Some(guard) = guard {
            statements.insert(0, guard);
        }
        let body = match node.body {
            Some(body) => ast::BlockExpression {
                loc: body.loc,
                statements: vec![statements, body.statements].concat(),
            },
            None => ast::BlockExpression {
                loc: node.loc,
                statements,
            },
        };
        ast::ForInExpression {
            loc: node.loc,
            pattern: Some(element.into()),
            iterable: node.iterable,
            body: Some(body),
        }
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
            self.check_assigned_type(ty, curr, stmt.loc);
        }

        self.intern(OptionType { some: ty })
    }

    fn break_type(&mut self, stmt: &ir::BreakStatement) -> TypeId {
        stmt.expression.as_ref().map_or(TypeStore::UNIT, |e| e.ty())
    }
}
