use tine_ast::*;
use tine_common::locations::Locatable;

use crate::{DiagnosticKind, Parser};

impl Parser<'_> {
    pub fn parse_pattern(&mut self) -> Option<Pattern> {
        self.parse_expression_with_block()
            .map(|e| self.expr_to_pattern(e))
    }

    pub fn expr_to_pattern(&mut self, expr: Expression) -> Pattern {
        match expr {
            Expression::Identifier(id) => Pattern::Identifier(id.into()),
            Expression::BooleanLiteral(lit) => Pattern::Literal(LiteralPattern::Boolean(lit)),
            Expression::Call(c) => self.call_to_pattern(c),
            Expression::StringLiteral(lit) => Pattern::Literal(LiteralPattern::String(lit)),
            Expression::IntLiteral(lit) => Pattern::Literal(LiteralPattern::Integer(lit)),
            Expression::FloatLiteral(lit) => Pattern::Literal(LiteralPattern::Float(lit)),
            Expression::Path(p) => self.path_to_pattern(p),
            Expression::Struct(s) => Pattern::Struct(self.struct_to_pattern(s)),
            Expression::Tuple(tuple) => Pattern::Tuple(self.tuple_to_pattern(tuple)),
            Expression::Unary(unary) => self.unary_expr_to_pattern(unary),
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

        let args = call
            .args
            .into_iter()
            .map(|e| self.expr_to_pattern(e))
            .collect::<Vec<_>>();

        Pattern::Call(CallPattern {
            loc: call.loc,
            path,
            args,
        })
    }

    fn path_to_pattern(&mut self, path: PathExpression) -> Pattern {
        if path.segments.iter().any(|s| s.generic_args.is_some()) {
            return Pattern::Invalid(InvalidPattern { loc: path.loc });
        }
        Pattern::Path(path)
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

    fn unary_expr_to_pattern(&mut self, unary: UnaryExpression) -> Pattern {
        if unary.operator != UnaryOperator::Mut {
            self.error(DiagnosticKind::InvalidPattern, unary.loc());
            return Pattern::Invalid(InvalidPattern { loc: unary.loc });
        }
        let Some(path) = unary.operand else {
            self.error(DiagnosticKind::InvalidPattern, unary.loc());
            return Pattern::Invalid(InvalidPattern { loc: unary.loc });
        };
        let Expression::Path(path) = *path else {
            self.error(DiagnosticKind::InvalidPattern, path.loc());
            return Pattern::Invalid(InvalidPattern { loc: unary.loc });
        };
        match self.path_to_pattern(path) {
            Pattern::Identifier(mut identifier) => {
                identifier.mutable = true;
                identifier.loc = unary.loc;
                identifier.into()
            }
            p => p,
        }
    }
}
