use pest::iterators::{Pair, Pairs};

use crate::{
    ast,
    parser::{parser::Rule, ParserEngine},
    DiagnosticKind, Location,
};

impl ParserEngine {
    pub fn parse_if_expression(&mut self, pair: Pair<'_, Rule>) -> ast::IfExpression {
        debug_assert_eq!(pair.as_rule(), Rule::if_expression);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let condition = Box::new(self.parse_condition(&mut inner, loc));
        let consequent = Box::new(self.parse_consequent(&mut inner, condition.loc().increment()));
        let alternate = inner
            .next()
            .map(|pair| Box::new(self.parse_alternate(pair)));
        ast::IfExpression {
            loc,
            condition,
            consequent,
            alternate,
        }
    }

    /// Parse if expressions that use pattern matching as their condition
    pub fn parse_if_pat_expression(&mut self, pair: Pair<'_, Rule>) -> ast::IfPatExpression {
        assert!(pair.as_rule() == Rule::if_decl_expression);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let pattern = Box::new(self.parse_pattern(inner.next().unwrap()));
        let scrutinee = Box::new(self.parse_expression(inner.next().unwrap()));
        let consequent = Box::new(self.parse_block(inner.next().unwrap()));
        let alternate = inner
            .next()
            .map(|pair| Box::new(self.parse_alternate(pair)));
        ast::IfPatExpression {
            loc,
            pattern,
            scrutinee,
            consequent,
            alternate,
        }
    }

    fn parse_condition(&mut self, pairs: &mut Pairs<'_, Rule>, loc: Location) -> ast::Expression {
        if pairs.peek().unwrap().as_rule() == Rule::condition {
            self.parse_expression(pairs.next().unwrap())
        } else {
            self.error(
                DiagnosticKind::MissingExpression,
                loc.decrement().increment().increment().increment(),
            );
            ast::Expression::Empty
        }
    }

    fn parse_consequent(
        &mut self,
        pairs: &mut Pairs<'_, Rule>,
        loc: Location,
    ) -> ast::BlockExpression {
        match pairs.peek() {
            Some(pair) if pair.as_rule() == Rule::block => self.parse_block(pairs.next().unwrap()),
            _ => {
                self.error(DiagnosticKind::MissingConsequent, loc);
                ast::BlockExpression {
                    loc,
                    statements: vec![],
                }
            }
        }
    }

    fn parse_alternate(&mut self, pair: Pair<'_, Rule>) -> ast::Alternate {
        assert!(pair.as_rule() == Rule::alternate);
        let pair = pair.into_inner().next().unwrap();
        match pair.as_rule() {
            Rule::block => self.parse_block(pair).into(),
            Rule::if_expression => self.parse_if_expression(pair).into(),
            Rule::if_decl_expression => self.parse_if_pat_expression(pair).into(),
            rule => unreachable!("unexpected rule {:?}", rule),
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
    fn test_parse_if_expression() {
        let input = "if true {}";
        let expected = ast::Expression::If(ast::IfExpression {
            loc: Location::new(0, Span::new(0, input.len() as u32)),
            condition: Box::new(ast::Expression::BooleanLiteral(ast::BooleanLiteral {
                loc: Location::new(0, Span::new(3, 7)),
                value: true,
            })),
            consequent: Box::new(ast::BlockExpression {
                loc: Location::new(0, Span::new(8, 10)),
                statements: vec![],
            }),
            alternate: None,
        });
        let (result, diagnostics) = parse_expression_input(input);
        assert_eq!(result, expected);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_parse_if_expression_missing_condition() {
        let input = "if {}";
        let expected = ast::Expression::If(ast::IfExpression {
            loc: Location::new(0, Span::new(0, input.len() as u32)),
            condition: Box::new(ast::Expression::Empty),
            consequent: Box::new(ast::BlockExpression {
                loc: Location::new(0, Span::new(3, 5)),
                statements: vec![],
            }),
            alternate: None,
        });
        let (result, diagnostics) = parse_expression_input(input);
        assert_eq!(result, expected);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].kind, DiagnosticKind::MissingExpression)
    }

    #[test]
    fn test_parse_if_expression_missing_block() {
        let input = "if true";
        let expected = ast::Expression::If(ast::IfExpression {
            loc: Location::new(0, Span::new(0, input.len() as u32)),
            condition: Box::new(ast::Expression::BooleanLiteral(ast::BooleanLiteral {
                loc: Location::new(0, Span::new(3, 7)),
                value: true,
            })),
            consequent: Box::new(ast::BlockExpression {
                loc: Location::new(0, Span::new(7, 8)),
                statements: vec![],
            }),
            alternate: None,
        });
        let (result, diagnostics) = parse_expression_input(input);
        assert_eq!(result, expected);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].kind, DiagnosticKind::MissingConsequent);
    }

    #[test]
    fn test_parse_if_expression_with_alternate() {
        let input = "if true {} else {}";
        let expected = ast::Expression::If(ast::IfExpression {
            loc: Location::new(0, Span::new(0, input.len() as u32)),
            condition: Box::new(ast::Expression::BooleanLiteral(ast::BooleanLiteral {
                loc: Location::new(0, Span::new(3, 7)),
                value: true,
            })),
            consequent: Box::new(ast::BlockExpression {
                loc: Location::new(0, Span::new(8, 10)),
                statements: vec![],
            }),
            alternate: Some(Box::new(ast::Alternate::Block(ast::BlockExpression {
                loc: Location::new(0, Span::new(16, 18)),
                statements: vec![],
            }))),
        });
        let (result, diagnostics) = parse_expression_input(input);
        assert_eq!(result, expected);
        assert!(diagnostics.is_empty());
    }
}
