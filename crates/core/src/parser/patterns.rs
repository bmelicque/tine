use crate::{ast, parser::Parser, DiagnosticKind};

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
            ast::Expression::ConstructorLiteral(lit) => {
                ast::Pattern::Constructor(ast::ConstructorPattern {
                    loc: lit.loc,
                    qualifiers: lit.qualifiers,
                    constructor: lit.constructor,
                    body: match lit.body {
                        Some(ast::ConstructorBody::Struct(s)) => Some(
                            ast::ConstructorPatternBody::Struct(ast::StructPatternBody {
                                loc: s.loc,
                                fields: s
                                    .fields
                                    .into_iter()
                                    .map(|f| self.struct_field_to_pattern(f))
                                    .collect(),
                            }),
                        ),
                        Some(ast::ConstructorBody::Tuple(t)) => {
                            Some(self.tuple_to_pattern(t).into())
                        }
                        None => None,
                    },
                })
            }
            ast::Expression::Tuple(tuple) => ast::Pattern::Tuple(self.tuple_to_pattern(tuple)),
            _ => {
                self.error(DiagnosticKind::InvalidPattern, expr.loc());
                ast::Pattern::Invalid(ast::InvalidPattern { loc: expr.loc() })
            }
        }
    }

    fn tuple_to_pattern(&mut self, tuple: ast::TupleExpression) -> ast::TuplePattern {
        ast::TuplePattern {
            loc: tuple.loc,
            elements: tuple
                .elements
                .into_iter()
                .map(|e| self.expr_to_pattern(e))
                .collect(),
        }
    }

    fn struct_field_to_pattern(&mut self, field: ast::ConstructorField) -> ast::StructPatternField {
        let identifier = match field.key {
            Some(ast::ConstructorKey::Name(ident)) => Some(ident),
            Some(ast::ConstructorKey::MapKey(key)) => {
                self.error(DiagnosticKind::InvalidPattern, key.loc());
                None
            }
            None => None,
        };
        ast::StructPatternField {
            loc: field.loc,
            identifier,
            pattern: field.value.map(|v| self.expr_to_pattern(v)),
        }
    }
}
