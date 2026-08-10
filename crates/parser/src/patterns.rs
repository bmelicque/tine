use tine_ast::*;
use tine_common::locations::Locatable;

use crate::{DiagnosticKind, Parser};

impl Parser<'_> {
    pub fn parse_pattern(&mut self) -> Option<Pattern> {
        self.parse_expression().map(|e| self.expr_to_pattern(e))
    }

    pub fn expr_to_pattern(&mut self, expr: Expression) -> Pattern {
        match expr {
            Expression::Identifier(id) => Pattern::Identifier(id),
            Expression::BooleanLiteral(lit) => Pattern::Literal(LiteralPattern::Boolean(lit)),
            Expression::Call(c) => self.call_to_pattern(c),
            Expression::StringLiteral(lit) => Pattern::Literal(LiteralPattern::String(lit)),
            Expression::IntLiteral(lit) => Pattern::Literal(LiteralPattern::Integer(lit)),
            Expression::FloatLiteral(lit) => Pattern::Literal(LiteralPattern::Float(lit)),
            Expression::Path(p) => self.path_to_pattern(p),
            Expression::Struct(s) => Pattern::Struct(self.struct_to_pattern(s)),
            Expression::Tuple(tuple) => Pattern::Tuple(self.tuple_to_pattern(tuple)),
            Expression::Unary(unary)
                if unary.operator == UnaryOperator::Mut
                    && matches!(unary.operand.as_deref(), Some(Expression::Path(_))) =>
            {
                let Expression::Path(path) = *unary.operand.unwrap() else {
                    unreachable!()
                };
                match self.path_to_pattern(path) {
                    Pattern::Identifier(identifier) => {
                        Pattern::MutIdentifier(MutIdentifierPattern {
                            loc: unary.loc,
                            identifier,
                        })
                    }
                    p => p,
                }
            }
            _ => {
                self.error(DiagnosticKind::InvalidPattern, expr.loc());
                Pattern::Invalid(InvalidPattern { loc: expr.loc() })
            }
        }
    }

    fn call_to_pattern(&mut self, call: CallExpression) -> Pattern {
        let path = match call.callee.map(|c| *c) {
            Some(Expression::Path(p)) => p,
            _ => return Pattern::Invalid(InvalidPattern { loc: call.loc }),
        };

        let elements = call
            .args
            .into_iter()
            .map(|e| match e {
                CallArgument::Callback(c) => Pattern::Invalid(InvalidPattern { loc: c.loc }),
                CallArgument::Expression(e) => self.expr_to_pattern(e),
            })
            .collect::<Vec<_>>();

        Pattern::Tuple(TuplePattern {
            loc: call.loc,
            path: Some(path),
            elements,
        })
    }

    fn path_to_pattern(&mut self, mut path: PathExpression) -> Pattern {
        if path.segments.len() != 1 {
            return Pattern::Invalid(InvalidPattern { loc: path.loc });
        }
        let segment = path.segments.pop().unwrap();
        if segment.generic_args.is_some() {
            return Pattern::Invalid(InvalidPattern { loc: path.loc });
        }
        Pattern::Identifier(segment.ident)
    }

    fn struct_to_pattern(&mut self, struct_: StructExpression) -> StructPattern {
        StructPattern {
            loc: struct_.loc,
            path: struct_.constructor,
            fields: struct_
                .fields
                .into_iter()
                .map(|f| self.struct_field_to_pattern(f))
                .collect(),
        }
    }

    fn tuple_to_pattern(&mut self, tuple: TupleExpression) -> TuplePattern {
        TuplePattern {
            loc: tuple.loc,
            path: None,
            elements: tuple
                .elements
                .into_iter()
                .map(|e| self.expr_to_pattern(e))
                .collect(),
        }
    }

    fn struct_field_to_pattern(&mut self, field: StructExprField) -> StructPatternField {
        let identifier = match field.key {
            Some(StructExprFieldKey::Name(ident)) => Some(ident.into()),
            Some(StructExprFieldKey::MapKey(key)) => {
                self.error(DiagnosticKind::InvalidPattern, key.loc());
                None
            }
            None => None,
        };
        StructPatternField {
            loc: field.loc,
            identifier,
            pattern: field.value.map(|v| self.expr_to_pattern(v)),
        }
    }
}
