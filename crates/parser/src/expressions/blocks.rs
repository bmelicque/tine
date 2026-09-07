use tine_ast::*;
use tine_common::locations::Location;

use crate::{tokens::Token, Parser};

impl Parser<'_> {
    pub fn parse_block(&mut self) -> BlockExpression {
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

        self.validate_block_functions(&statements);
        last_function_to_expr(&mut statements);

        BlockExpression {
            loc: Location::merge(self.localize(start_range), self.localize(end_range)),
            statements,
        }
    }

    fn validate_block_functions(&mut self, statements: &[Statement]) {
        if statements.is_empty() {
            return;
        }
        statements
            .into_iter()
            .rev()
            .skip(1)
            .rev()
            .filter_map(|stmt| stmt.as_function())
            .for_each(|f| {
                self.validate_function_definition(f);
            });
    }
}

fn last_function_to_expr(statements: &mut Vec<Statement>) {
    let Some(last) = statements.pop() else {
        return;
    };

    let last = match last {
        Statement::Function(f) => Statement::Expression(ExpressionStatement {
            expression: Box::new(f.definition.into()),
        }),
        last => last,
    };
    statements.push(last);
}

#[cfg(test)]
mod tests {
    use tine_common::locations::Span;

    use crate::test_utils::{parse_expression, test_expression, ExpressionTest};

    use super::*;

    #[test]
    fn test_parse_block() {
        test_expression(ExpressionTest {
            input: "{}",
            expected: Expression::Block(BlockExpression {
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
            expected: Expression::Block(BlockExpression {
                loc: Location::new(0, Span::new(0, 5)),
                statements: vec![Statement::Expression(ExpressionStatement {
                    expression: Box::new(Expression::IntLiteral(IntLiteral {
                        loc: Location::new(0, Span::new(2, 3)),
                        value: 1,
                    })),
                })],
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn parse_block_returns_fn() {
        let expr = parse_expression("{\nfn() {}\n}").expect("expected no errors");
        let block = expr.as_block().expect("expected a block");
        let last = block.statements.last().expect("expected a statement");
        let expr = last
            .as_expression()
            .expect("expected an expression statement");
        expr.expression
            .as_function()
            .expect("expected a function expression");
    }
}
