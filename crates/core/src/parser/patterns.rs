use crate::{ast, parser::Parser, DiagnosticKind, Location};

impl Parser<'_> {
    pub fn parse_pattern(&mut self) -> Option<ast::Pattern> {
        self.parse_expression().map(|e| self.expr_to_pattern(e))
    }

    pub fn expr_to_pattern(&mut self, expr: ast::Expression) -> ast::Pattern {
        match expr {
            ast::Expression::Identifier(id) => ast::Pattern::Identifier(ast::IdentifierPattern(id)),
            ast::Expression::BooleanLiteral(lit) => {
                ast::Pattern::Literal(ast::LiteralPattern::Boolean(lit))
            }
            ast::Expression::StringLiteral(lit) => {
                ast::Pattern::Literal(ast::LiteralPattern::String(lit))
            }
            ast::Expression::IntLiteral(lit) => {
                ast::Pattern::Literal(ast::LiteralPattern::Integer(lit))
            }
            ast::Expression::FloatLiteral(lit) => {
                ast::Pattern::Literal(ast::LiteralPattern::Float(lit))
            }
            ast::Expression::CompositeLiteral(lit) => match lit {
                ast::CompositeLiteral::Struct(lit) => ast::Pattern::Struct(ast::StructPattern {
                    loc: lit.loc,
                    ty: Box::new(lit.ty),
                    fields: lit
                        .fields
                        .into_iter()
                        .map(|f| self.struct_field_to_pattern(f))
                        .collect(),
                }),
                ast::CompositeLiteral::Variant(lit) => ast::Pattern::Variant(ast::VariantPattern {
                    loc: lit.loc,
                    ty: Box::new(lit.ty),
                    name: lit.name,
                    body: lit.body.map(|b| self.variant_body_to_pattern(b)),
                }),
                _ => {
                    self.error(DiagnosticKind::InvalidPattern, lit.loc());
                    ast::Pattern::Invalid(ast::InvalidPattern { loc: lit.loc() })
                }
            },
            ast::Expression::Tuple(tuple) => ast::Pattern::Tuple(ast::TuplePattern {
                loc: tuple.loc,
                elements: tuple
                    .elements
                    .into_iter()
                    .map(|e| self.expr_to_pattern(e))
                    .collect(),
            }),
            _ => {
                self.error(DiagnosticKind::InvalidPattern, expr.loc());
                ast::Pattern::Invalid(ast::InvalidPattern { loc: expr.loc() })
            }
        }
    }

    fn struct_field_to_pattern(
        &mut self,
        field: ast::StructLiteralField,
    ) -> ast::StructPatternField {
        ast::StructPatternField {
            loc: field.loc,
            identifier: field.prop,
            pattern: field.value.map(|v| self.expr_to_pattern(v)),
        }
    }

    fn variant_body_to_pattern(
        &mut self,
        body: ast::VariantLiteralBody,
    ) -> ast::VariantPatternBody {
        match body {
            ast::VariantLiteralBody::Struct(b) => ast::VariantPatternBody::Struct(
                b.into_iter()
                    .map(|f| self.struct_field_to_pattern(f))
                    .collect(),
            ),
            ast::VariantLiteralBody::Tuple(b) => {
                let loc = Location::merge(b.first().unwrap().loc(), b.last().unwrap().loc());
                let elements = b
                    .into_iter()
                    .map(|e| self.expr_or_anonymous_to_pattern(e))
                    .collect();
                ast::VariantPatternBody::Tuple(ast::TuplePattern { loc, elements })
            }
        }
    }

    fn expr_or_anonymous_to_pattern(&mut self, expr: ast::ExpressionOrAnonymous) -> ast::Pattern {
        match expr {
            ast::ExpressionOrAnonymous::Expression(e) => self.expr_to_pattern(e),
            _ => {
                self.error(DiagnosticKind::InvalidPattern, expr.loc());
                ast::Pattern::Invalid(ast::InvalidPattern { loc: expr.loc() })
            }
        }
    }
}
