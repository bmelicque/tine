use crate::{ast, Location};

use super::TypeChecker;

pub struct DesugaredPattern {
    pub test: Option<ast::Expression>,
    pub bindings: Vec<(ast::Identifier, ast::Expression)>,
}
impl DesugaredPattern {
    pub fn new() -> Self {
        Self {
            test: None,
            bindings: Vec::new(),
        }
    }

    pub fn merge(a: DesugaredPattern, b: DesugaredPattern) -> Self {
        let test = match (a.test, b.test) {
            (Some(a), Some(b)) => Some(ast::Expression::Binary(ast::BinaryExpression {
                loc: Location::merge(a.loc(), b.loc()),
                left: Some(Box::new(a)),
                operator: ast::BinaryOperator::LAnd,
                right: Some(Box::new(b)),
            })),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        let assignments = vec![a.bindings, b.bindings].concat();
        Self {
            test,
            bindings: assignments,
        }
    }
}

impl TypeChecker<'_> {
    /// Desugar pattern-matching into a test and the corresponding bindings.
    ///
    /// For example, `if const User{ age, name: "John" } = user { ... }` will be transformed into:
    /// ```tine
    /// if user.name == "John" {
    ///     const age = user.age
    ///     ...
    /// }
    /// ```
    ///
    /// No type-checking is done here, so the type constructor (if any) should be checked before desugaring.
    pub fn desugar_pattern(
        &mut self,
        pattern: ast::Pattern,
        against: ast::Expression,
        mutable: bool,
    ) -> DesugaredPattern {
        match pattern {
            ast::Pattern::Constructor(pat) => {
                self.desugar_constructor_pattern(pat, against, mutable)
            }
            ast::Pattern::Invalid { .. } => DesugaredPattern::new(),
            ast::Pattern::Identifier(id) => DesugaredPattern {
                test: None,
                bindings: vec![(id.0, against)],
            },
            ast::Pattern::Literal(pat) => self.desugar_literal_pattern(pat, against),
            ast::Pattern::Tuple(pat) => self.desugar_tuple_pattern(pat, against, mutable),
        }
    }

    fn desugar_literal_pattern(
        &mut self,
        pattern: ast::LiteralPattern,
        against: ast::Expression,
    ) -> DesugaredPattern {
        let got: ast::Expression = match pattern {
            ast::LiteralPattern::Boolean(b) => b.into(),
            ast::LiteralPattern::Float(f) => f.into(),
            ast::LiteralPattern::Integer(i) => i.into(),
            ast::LiteralPattern::String(s) => s.into(),
        };

        let test = ast::Expression::Binary(ast::BinaryExpression {
            loc: got.loc(),
            left: Some(Box::new(against.clone())),
            operator: ast::BinaryOperator::EqEq,
            right: Some(Box::new(got)),
        });

        DesugaredPattern {
            test: Some(test),
            bindings: vec![],
        }
    }

    fn desugar_tuple_pattern(
        &mut self,
        pattern: ast::TuplePattern,
        against: ast::Expression,
        mutable: bool,
    ) -> DesugaredPattern {
        let mut desugared = DesugaredPattern::new();
        for (i, pattern) in pattern.elements.into_iter().enumerate() {
            let against = ast::Expression::Member(ast::MemberExpression {
                loc: against.loc(),
                object: Some(Box::new(against.clone())),
                prop: Some(ast::MemberProp::Index(ast::IntLiteral {
                    loc: against.loc(),
                    value: i as i64,
                })),
            });
            desugared =
                DesugaredPattern::merge(desugared, self.desugar_pattern(pattern, against, mutable))
        }
        desugared
    }

    fn desugar_constructor_pattern(
        &mut self,
        pattern: ast::ConstructorPattern,
        against: ast::Expression,
        mutable: bool,
    ) -> DesugaredPattern {
        let Some(body) = pattern.body else {
            return DesugaredPattern::new();
        };
        let desugared_body = match body {
            ast::ConstructorPatternBody::Struct(s) => {
                self.desugar_struct_pattern(s, against.clone(), mutable)
            }
            ast::ConstructorPatternBody::Tuple(t) => {
                self.desugar_tuple_pattern(t, against.clone(), mutable)
            }
        };
        match pattern.constructor {
            ast::Constructor::Variant(v) => {
                let variant_test = ast::Expression::TypeMatch(ast::TypeMatch {
                    loc: v.loc,
                    expression: Some(Box::new(against)),
                    constructor: v,
                });

                DesugaredPattern::merge(
                    DesugaredPattern {
                        test: Some(variant_test),
                        bindings: vec![],
                    },
                    desugared_body,
                )
            }
            _ => desugared_body,
        }
    }

    fn desugar_struct_pattern(
        &mut self,
        pattern: ast::StructPatternBody,
        against: ast::Expression,
        mutable: bool,
    ) -> DesugaredPattern {
        let mut desugared = DesugaredPattern::new();
        for pattern in pattern.fields {
            let Some(identifier) = pattern.identifier else {
                continue;
            };
            let against = ast::Expression::Member(ast::MemberExpression {
                loc: against.loc(),
                object: Some(Box::new(against.clone())),
                prop: Some(ast::MemberProp::FieldName(identifier.clone())),
            });
            let current = match pattern.pattern {
                Some(pattern) => self.desugar_pattern(pattern, against, mutable),
                None => DesugaredPattern {
                    test: None,
                    bindings: vec![(identifier, against)],
                },
            };
            desugared = DesugaredPattern::merge(desugared, current);
        }
        desugared
    }
}
