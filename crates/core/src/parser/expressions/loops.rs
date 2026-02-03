use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, ParserEngine},
    DiagnosticKind,
};

impl ParserEngine {
    pub fn parse_loop(&mut self, pair: Pair<'_, Rule>) -> ast::Loop {
        assert_eq!(pair.as_rule(), Rule::loop_expression);
        let pair = pair.into_inner().next().unwrap();
        match pair.as_rule() {
            Rule::for_expression => self.parse_for_expression(pair).into(),
            Rule::for_in_expression => self.parse_for_in_expression(pair).into(),
            rule => unreachable!("unexpected rule {:?}", rule),
        }
    }

    fn parse_for_expression(&mut self, pair: Pair<'_, Rule>) -> ast::ForExpression {
        debug_assert_eq!(pair.as_rule(), Rule::for_expression);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let condition = if inner.peek().unwrap().as_rule() == Rule::condition {
            Box::new(self.parse_expression(inner.next().unwrap()))
        } else {
            Box::new(ast::Expression::Empty)
        };
        let body = match inner.next() {
            Some(pair) => self.parse_block(pair),
            None => {
                self.error(DiagnosticKind::MissingConsequent, loc);
                ast::BlockExpression {
                    loc: loc.increment(),
                    statements: vec![],
                }
            }
        };
        ast::ForExpression {
            loc,
            condition,
            body,
        }
    }

    fn parse_for_in_expression(&mut self, pair: Pair<'_, Rule>) -> ast::ForInExpression {
        assert_eq!(pair.as_rule(), Rule::for_in_expression);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let pattern = Box::new(self.parse_pattern(inner.next().unwrap()));
        let iterable = Box::new(self.parse_expression(inner.next().unwrap()));
        let body = self.parse_block(inner.next().unwrap());
        ast::ForInExpression {
            loc,
            pattern,
            iterable,
            body,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        parser::parser::{Rule, TineParser},
        Diagnostic, DiagnosticKind, Location, Span,
    };
    use pest::Parser;

    fn parse_expression_input(input: &'static str) -> (ast::Expression, Vec<Diagnostic>) {
        let pair = TineParser::parse(Rule::expression, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new(0);
        (
            parser_engine.parse_expression(pair),
            parser_engine.diagnostics,
        )
    }

    #[test]
    fn test_parse_for_loop() {
        let input = "for i >= 0 {}";
        let expected = ast::Expression::Loop(ast::Loop::For(ast::ForExpression {
            loc: Location::new(0, Span::new(0, input.len() as u32)),
            condition: Box::new(ast::Expression::Binary(ast::BinaryExpression {
                loc: Location::new(0, Span::new(4, 10)),
                operator: ast::BinaryOperator::Geq,
                left: Box::new(ast::Expression::Identifier(ast::Identifier {
                    loc: Location::new(0, Span::new(4, 5)),
                    text: "i".to_string(),
                })),
                right: Box::new(ast::Expression::IntLiteral(ast::IntLiteral {
                    loc: Location::new(0, Span::new(9, 10)),
                    value: 0,
                })),
            })),
            body: ast::BlockExpression {
                loc: Location::new(0, Span::new(11, input.len() as u32)),
                statements: vec![],
            },
        }));
        let (actual, diagnostics) = parse_expression_input(input);
        assert_eq!(expected, actual);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn parse_for_loop_without_condition() {
        let input = "for {}";
        let expected = ast::Expression::Loop(ast::Loop::For(ast::ForExpression {
            loc: Location::new(0, Span::new(0, input.len() as u32)),
            condition: Box::new(ast::Expression::Empty),
            body: ast::BlockExpression {
                loc: Location::new(0, Span::new(4, input.len() as u32)),
                statements: vec![],
            },
        }));
        let (actual, diagnostics) = parse_expression_input(input);
        assert_eq!(expected, actual);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_parse_for_loop_missing_body() {
        let input = "for i >= 0";
        let expected = ast::Expression::Loop(ast::Loop::For(ast::ForExpression {
            loc: Location::new(0, Span::new(0, input.len() as u32)),
            condition: Box::new(ast::Expression::Binary(ast::BinaryExpression {
                loc: Location::new(0, Span::new(4, 10)),
                operator: ast::BinaryOperator::Geq,
                left: Box::new(ast::Expression::Identifier(ast::Identifier {
                    loc: Location::new(0, Span::new(4, 5)),
                    text: "i".to_string(),
                })),
                right: Box::new(ast::Expression::IntLiteral(ast::IntLiteral {
                    loc: Location::new(0, Span::new(9, 10)),
                    value: 0,
                })),
            })),
            body: ast::BlockExpression {
                loc: Location::new(0, Span::new(10, 11 as u32)),
                statements: vec![],
            },
        }));
        let (actual, diagnostics) = parse_expression_input(input);
        assert_eq!(expected, actual);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].kind, DiagnosticKind::MissingConsequent);
    }
}
