use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

use crate::ast;

use super::{
    utils::{create_ident, true_lit},
    CodeGenerator,
};

impl CodeGenerator {
    pub fn pat_or_expr_to_swc(&mut self, node: ast::PatternExpression) -> swc::PatOrExpr {
        match node {
            ast::PatternExpression::Expression(e) => Box::new(self.expr_to_swc(e)).into(),
            ast::PatternExpression::Pattern(p) => Box::new(self.pattern_to_swc(p)).into(),
        }
    }

    pub fn pattern_to_swc(&mut self, node: ast::Pattern) -> swc::Pat {
        match node {
            ast::Pattern::Identifier(pattern) => swc::Pat::Ident(swc::BindingIdent {
                id: create_ident(pattern.span.as_str()),
                type_ann: None,
            }),
            ast::Pattern::StructPattern(pattern) => self.struct_pattern_to_swc(pattern).into(),
            ast::Pattern::Tuple(pattern) => self.tuple_pattern_to_swc(pattern).into(),
        }
    }

    fn struct_pattern_to_swc(&mut self, node: ast::StructPattern) -> swc::ObjectPat {
        swc::ObjectPat {
            span: DUMMY_SP,
            props: node
                .fields
                .into_iter()
                .map(|field| self.struct_pattern_field_to_swc(field))
                .collect(),
            optional: false,
            type_ann: None,
        }
    }

    /// FIXME: handle values (for example, Struct(field: true))
    fn struct_pattern_field_to_swc(&mut self, node: ast::StructPatternField) -> swc::ObjectPatProp {
        match node.pattern {
            Some(pattern) => swc::KeyValuePatProp {
                key: create_ident(&node.identifier).into(),
                value: Box::new(self.pattern_to_swc(pattern)),
            }
            .into(),
            None => swc::AssignPatProp {
                span: DUMMY_SP,
                key: create_ident(&node.identifier).into(),
                value: None,
            }
            .into(),
        }
    }

    fn tuple_pattern_to_swc(&mut self, node: ast::TuplePattern) -> swc::ArrayPat {
        swc::ArrayPat {
            span: DUMMY_SP,
            elems: node
                .elements
                .into_iter()
                .map(|element| Some(self.pattern_to_swc(element)))
                .collect(),
            optional: false,
            type_ann: None,
        }
    }

    /// Create the JS test expression that will validate if the pattern is matched.
    /// Also provide needed JS declarations, resulting from said test.
    ///
    /// For example: `if (0, value) := tuple { ... }` will:
    /// - check that `tuple[0] === 0`
    /// - declare `let value = tuple[0]`
    /// For a result being: `if (tuple[0] === 0) { let value = tuple[0]; ... }`
    pub fn pattern_to_swc_test(
        &mut self,
        pattern: &ast::Pattern,
        against: &ast::Expression,
    ) -> swc::Expr {
        match pattern {
            ast::Pattern::Identifier(_) => true_lit(),
            ast::Pattern::StructPattern(s) => self.struct_pattern_to_swc_test(s, against),
            ast::Pattern::Tuple(t) => self.tuple_pattern_to_swc_test(t, against),
        }
    }

    fn struct_pattern_to_swc_test(
        &mut self,
        pattern: &ast::StructPattern,
        against: &ast::Expression,
    ) -> swc::Expr {
        let tests: Vec<swc::Expr> = pattern
            .fields
            .iter()
            .filter(|field| field.pattern.is_some())
            .map(|field| {
                let against = ast::FieldAccessExpression {
                    span: against.as_span(),
                    object: Box::new(against.clone()),
                    prop: ast::Identifier {
                        span: pest::Span::new(
                            Box::leak(field.identifier.clone().into_boxed_str()),
                            0,
                            field.identifier.len(),
                        )
                        .unwrap(),
                    },
                };
                self.pattern_to_swc_test(&field.pattern.as_ref().unwrap(), &against.into())
            })
            .collect();
        let Some(test) = tests.first() else {
            return true_lit();
        };
        let mut test = test.clone();
        for t in tests.into_iter().skip(1) {
            test = swc::Expr::Bin(swc::BinExpr {
                span: DUMMY_SP,
                op: swc::BinaryOp::LogicalAnd,
                left: Box::new(test),
                right: Box::new(t),
            });
        }
        test
    }

    fn tuple_pattern_to_swc_test(
        &mut self,
        pattern: &ast::TuplePattern,
        against: &ast::Expression,
    ) -> swc::Expr {
        let tests: Vec<swc::Expr> = pattern
            .elements
            .iter()
            .enumerate()
            .filter(|(_, el)| !el.is_identifier())
            .map(|(i, el)| {
                let against = ast::TupleIndexingExpression {
                    span: against.as_span(),
                    tuple: Box::new(against.clone()),
                    index: ast::NumberLiteral {
                        span: against.as_span(),
                        value: i as f64,
                    },
                };
                self.pattern_to_swc_test(el, &against.into())
            })
            .collect();
        let Some(test) = tests.first() else {
            return true_lit();
        };
        let mut test = test.clone();
        for t in tests.into_iter().skip(1) {
            test = swc::Expr::Bin(swc::BinExpr {
                span: DUMMY_SP,
                op: swc::BinaryOp::LogicalAnd,
                left: Box::new(test),
                right: Box::new(t),
            });
        }
        test
    }
}
