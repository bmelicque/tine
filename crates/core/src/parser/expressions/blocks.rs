use crate::{
    ast,
    parser::{tokens::Token, Parser},
    Location,
};

impl Parser<'_> {
    pub fn parse_block(&mut self) -> ast::BlockExpression {
        let start_range = self.eat(&[Token::LBrace]);

        let mut statements = Vec::new();
        while let Some((Ok(token), _)) = self.tokens.peek().cloned() {
            match token.clone() {
                Token::Newline => {
                    self.tokens.next();
                }
                Token::RBrace => break,
                _ => {
                    if let Some(element) = self.parse_statement() {
                        statements.push(element);
                    }
                    match self.tokens.peek() {
                        Some((Ok(Token::Newline | Token::RBrace), _)) => {}
                        _ => {
                            self.recover_before(&[Token::Newline, Token::RBrace], &[]);
                        }
                    }
                }
            }
        }

        let end_range = self.expect(Token::RBrace);

        ast::BlockExpression {
            loc: Location::merge(self.localize(start_range), self.localize(end_range)),
            statements,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        parser::test_utils::{test_expression, ExpressionTest},
        Span,
    };

    use super::*;

    #[test]
    fn test_parse_block() {
        test_expression(ExpressionTest {
            input: "{}",
            expected: ast::Expression::Block(ast::BlockExpression {
                loc: Location::new(0, Span::new(0, 2)),
                statements: vec![],
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn test_parse_block_with_statement() {
        test_expression(ExpressionTest {
            input: "{\n1\n}",
            expected: ast::Expression::Block(ast::BlockExpression {
                loc: Location::new(0, Span::new(0, 5)),
                statements: vec![ast::Statement::Expression(ast::ExpressionStatement {
                    expression: Box::new(ast::Expression::IntLiteral(ast::IntLiteral {
                        loc: Location::new(0, Span::new(2, 3)),
                        value: 1,
                    })),
                })],
            }),
            diagnostics: vec![],
        });
    }
}
