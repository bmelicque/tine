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

    /// Used to build a destructuring expression, like the `{ name }` part of `const { name } = user;`
    pub fn pattern_to_swc(&mut self, node: ast::Pattern) -> swc::Pat {
        match node {
            ast::Pattern::Identifier(pattern) => self.identifier_pattern_to_swc(pattern),
            ast::Pattern::Literal(pattern) => {
                // TODO: this is probably useless (investigate)
                swc::Pat::Expr(self.literal_pattern_to_swc(pattern).into())
            }
            ast::Pattern::Struct(pattern) => self.struct_pattern_to_swc(pattern.fields).into(),
            ast::Pattern::Tuple(pattern) => self.tuple_pattern_to_swc(pattern).into(),
            ast::Pattern::Variant(pattern) => self.variant_pattern_to_swc(pattern),
        }
    }

    pub fn identifier_pattern_to_swc(&mut self, node: ast::IdentifierPattern) -> swc::Pat {
        swc::Pat::Ident(swc::BindingIdent {
            id: create_ident(node.span.as_str()),
            type_ann: None,
        })
    }

    fn literal_pattern_to_swc(&mut self, node: ast::LiteralPattern) -> swc::Lit {
        match node {
            ast::LiteralPattern::Boolean(b) => swc::Lit::Bool(swc::Bool {
                span: DUMMY_SP,
                value: b.value,
            }),
            ast::LiteralPattern::Number(n) => swc::Lit::Num(swc::Number {
                span: DUMMY_SP,
                value: n.value,
                raw: None,
            }),
            ast::LiteralPattern::String(s) => swc::Lit::Str(swc::Str {
                span: DUMMY_SP,
                value: s.as_str().into(),
                raw: None,
            }),
        }
    }

    fn struct_pattern_to_swc(&mut self, fields: Vec<ast::StructPatternField>) -> swc::ObjectPat {
        swc::ObjectPat {
            span: DUMMY_SP,
            props: fields
                .into_iter()
                .filter(|field| match field.pattern {
                    Some(ast::Pattern::Literal(_)) => false,
                    _ => true,
                })
                .map(|field| self.struct_pattern_field_to_swc(field))
                .collect(),
            optional: false,
            type_ann: None,
        }
    }

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
                .filter(|element| !element.is_refutable())
                .map(|element| Some(self.pattern_to_swc(element)))
                .collect(),
            optional: false,
            type_ann: None,
        }
    }

    fn variant_pattern_to_swc(&mut self, node: ast::VariantPattern) -> swc::Pat {
        let Some(body) = node.body else {
            return swc::Pat::Ident(create_ident("__").into());
        };
        match body {
            ast::VariantPatternBody::Struct(ref fields) => {
                self.struct_pattern_to_swc(fields.to_vec()).into()
            }
            ast::VariantPatternBody::Tuple(ref body) => {
                self.tuple_pattern_to_swc(body.clone()).into()
            }
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
            ast::Pattern::Literal(l) => self.literal_pattern_to_swc_test(l, against),
            ast::Pattern::Struct(s) => self.struct_pattern_to_swc_test(&s.fields, against),
            ast::Pattern::Tuple(t) => self.tuple_pattern_to_swc_test(t, against),
            ast::Pattern::Variant(v) => self.variant_pattern_to_swc_test(v, against),
        }
    }

    fn literal_pattern_to_swc_test(
        &mut self,
        pattern: &ast::LiteralPattern,
        against: &ast::Expression,
    ) -> swc::Expr {
        swc::Expr::Bin(swc::BinExpr {
            span: DUMMY_SP,
            op: swc::BinaryOp::EqEqEq,
            left: Box::new(self.expr_to_swc(against.clone())),
            right: Box::new(self.literal_pattern_to_swc(pattern.clone()).into()),
        })
    }

    fn struct_pattern_to_swc_test(
        &mut self,
        fields: &Vec<ast::StructPatternField>,
        against: &ast::Expression,
    ) -> swc::Expr {
        let tests: Vec<swc::Expr> = fields
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
        self.tuple_to_swc_test_helper(pattern, |i| {
            ast::TupleIndexingExpression {
                span: against.as_span(),
                tuple: Box::new(against.clone()),
                index: ast::NumberLiteral {
                    span: against.as_span(),
                    value: i as f64,
                },
            }
            .into()
        })
    }

    fn variant_pattern_to_swc_test(
        &mut self,
        pattern: &ast::VariantPattern,
        against: &ast::Expression,
    ) -> swc::Expr {
        // TODO: new Enum(name, ...) => ty.__ === name
        let tag_test = swc::Expr::Bin(swc::BinExpr {
            span: DUMMY_SP,
            op: swc::BinaryOp::EqEqEq,
            left: Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(self.expr_to_swc(against.clone())),
                prop: swc::MemberProp::Ident(create_ident("__")),
            })),
            right: Box::new(swc::Expr::Lit(swc::Lit::Str(swc::Str {
                span: DUMMY_SP,
                value: pattern.name.clone().into(),
                raw: None,
            }))),
        });

        let Some(ref body) = pattern.body else {
            return tag_test;
        };

        let body_test = match body {
            ast::VariantPatternBody::Struct(ref fields) => {
                self.struct_pattern_to_swc_test(fields, against)
            }
            ast::VariantPatternBody::Tuple(tuple) => {
                self.tuple_variant_to_swc_test(&tuple, against)
            }
        };

        swc::Expr::Bin(swc::BinExpr {
            span: DUMMY_SP,
            op: swc::BinaryOp::LogicalAnd,
            left: Box::new(tag_test),
            right: Box::new(body_test),
        })
    }

    fn tuple_variant_to_swc_test(
        &mut self,
        pattern: &ast::TuplePattern,
        against: &ast::Expression,
    ) -> swc::Expr {
        self.tuple_to_swc_test_helper(pattern, |i| {
            let id = format!("_{}", i);
            let leaked_id: &'static str = Box::leak(id.into_boxed_str());
            ast::FieldAccessExpression {
                span: against.as_span(),
                object: Box::new(against.clone()),
                prop: ast::Identifier {
                    span: pest::Span::new(leaked_id, 0, leaked_id.len()).unwrap(),
                },
            }
            .into()
        })
    }

    fn tuple_to_swc_test_helper(
        &mut self,
        pattern: &ast::TuplePattern,
        mut get_sub_against: impl FnMut(usize) -> ast::Expression,
    ) -> swc::Expr {
        let tests: Vec<swc::Expr> = pattern
            .elements
            .iter()
            .enumerate()
            .filter(|(_, el)| !el.is_identifier())
            .map(|(i, el)| self.pattern_to_swc_test(el, &get_sub_against(i)))
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
