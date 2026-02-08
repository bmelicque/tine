use crate::{
    ast,
    parser2::{tokens::Token, Parser},
    DiagnosticKind, Location,
};

impl Parser<'_> {
    pub fn parse_loop_expression(&mut self) -> ast::Loop {
        let start_range = self.eat(&[Token::For]);
        let start_loc = self.localize(start_range);
        let expression = self.parse_expression_without_block();
        let mut loc = match &expression {
            Some(e) => Location::merge(start_loc, e.loc()),
            None => start_loc,
        };
        match self.tokens.peek() {
            Some((Ok(Token::In), _)) => {
                let pattern = expression.map(|e| self.expr_to_pattern(e));
                if pattern.is_none() {
                    let error_loc = self.next_loc();
                    self.error(DiagnosticKind::MissingPattern, error_loc);
                }
                self.tokens.next(); // consume the 'in' token
                let iterable = self.parse_expression_without_block();

                let body = self.parse_loop_body();
                loc = match &body {
                    Some(b) => Location::merge(loc, b.loc),
                    None => loc,
                };
                ast::Loop::ForIn(ast::ForInExpression {
                    loc,
                    pattern: pattern.map(|p| Box::new(p)),
                    iterable: iterable.map(|i| Box::new(i)),
                    body,
                })
            }
            _ => {
                let body = self.parse_loop_body();
                loc = match &body {
                    Some(b) => Location::merge(loc, b.loc),
                    None => loc,
                };
                ast::Loop::For(ast::ForExpression {
                    loc,
                    condition: expression.map(|e| Box::new(e)),
                    body,
                })
            }
        }
    }

    fn parse_loop_body(&mut self) -> Option<ast::BlockExpression> {
        match self.tokens.peek() {
            Some((Ok(Token::LBrace), _)) => Some(self.parse_block()),
            _ => {
                let error_loc = self.next_loc();
                self.error(DiagnosticKind::MissingConsequent, error_loc);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        parser2::test_utils::{test_expression, ExpressionTest},
        Span,
    };

    use super::*;

    #[test]
    fn parse_for_loop() {
        test_expression(ExpressionTest {
            input: "for true {}",
            expected: ast::Expression::Loop(ast::Loop::For(ast::ForExpression {
                loc: Location::new(0, Span::new(0, 11)),
                condition: Some(Box::new(ast::Expression::BooleanLiteral(
                    ast::BooleanLiteral {
                        loc: Location::new(0, Span::new(4, 8)),
                        value: true,
                    },
                ))),
                body: Some(ast::BlockExpression {
                    loc: Location::new(0, Span::new(9, 11)),
                    statements: vec![],
                }),
            })),
            diagnostics: vec![],
        });
    }
}
