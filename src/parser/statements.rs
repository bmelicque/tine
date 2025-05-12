use pest::iterators::Pair;

use crate::ast;

use super::{parser::Rule, ParserEngine};

impl ParserEngine {
    pub fn parse_statement(&mut self, pair: Pair<'static, Rule>) -> ast::Statement {
        match pair.as_rule() {
            Rule::statement => {
                let inner_pair = pair.into_inner().next().unwrap();
                self.parse_statement(inner_pair)
            }
            Rule::variable_declaration => self.parse_variable_declaration(pair).into(),
            Rule::assignment => self.parse_assignment(pair).into(),
            Rule::type_alias => self.parse_type_alias(pair).into(),
            Rule::return_statement => self.parse_return_statement(pair).into(),
            Rule::block => self.parse_block(pair).into(),
            Rule::expression_statement => self.parse_expression_statement(pair),
            _ => ast::Statement::Empty,
        }
    }

    fn parse_variable_declaration(
        &mut self,
        pair: Pair<'static, Rule>,
    ) -> ast::VariableDeclaration {
        let span = pair.as_span();
        let mut inner = pair.into_inner();

        let pattern = Box::new(self.parse_pattern(inner.next().unwrap()));
        let op = inner.next().unwrap().as_str().to_string().into();
        let value = Box::new(self.parse_expression(inner.next().unwrap()));

        ast::VariableDeclaration {
            span,
            pattern,
            op,
            value,
        }
    }

    fn parse_assignment(&mut self, pair: Pair<'static, Rule>) -> ast::Assignment {
        let span = pair.as_span();
        let mut inner = pair.into_inner();

        let name = inner.next().unwrap().as_str().to_string();
        let value = self.parse_expression(inner.next().unwrap());

        ast::Assignment { span, name, value }
    }

    fn parse_return_statement(&mut self, pair: Pair<'static, Rule>) -> ast::ReturnStatement {
        let span = pair.as_span();
        let value = pair
            .into_inner()
            .next()
            .map(|inner| self.parse_expression(inner))
            .map(Box::new);

        ast::ReturnStatement { span, value }
    }

    pub fn parse_block(&mut self, pair: Pair<'static, Rule>) -> ast::BlockStatement {
        let span = pair.as_span();
        let statements = pair
            .into_inner()
            .map(|pair| self.parse_statement(pair))
            .filter(|stmt| !stmt.is_empty())
            .collect();

        ast::BlockStatement { span, statements }
    }

    fn parse_expression_statement(&mut self, pair: Pair<'static, Rule>) -> ast::Statement {
        let Some(inner) = pair.into_inner().next() else {
            return ast::Statement::Empty;
        };
        match self.parse_expression(inner) {
            ast::Expression::Empty => ast::Statement::Empty,
            expression => ast::ExpressionStatement {
                expression: Box::new(expression),
            }
            .into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{MyLanguageParser, Rule};
    use pest::Parser;

    fn parse_statement_input(input: &'static str, rule: Rule) -> ast::Statement {
        let pair = MyLanguageParser::parse(rule, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new();
        parser_engine.parse_statement(pair)
    }

    #[test]
    fn test_parse_variable_declaration() {
        let input = "x := 42";
        let result = parse_statement_input(input, Rule::variable_declaration);

        match result {
            ast::Statement::VariableDeclaration(var_decl) => {
                match *var_decl.pattern {
                    ast::Pattern::Identifier(ast::IdentifierPattern { span })
                        if span.as_str() == "x" => {}
                    _ => panic!("Identifier pattern expected"),
                };
                assert_eq!(var_decl.op, ast::DeclarationOp::Mut);
                match *var_decl.value {
                    ast::Expression::NumberLiteral(literal) => assert_eq!(literal.value, 42.0),
                    _ => panic!("Expected NumberLiteral as variable value"),
                }
            }
            _ => panic!("Expected VariableDeclaration"),
        }
    }

    #[test]
    fn test_parse_assignment() {
        let input = "x = 42";
        let result = parse_statement_input(input, Rule::assignment);

        match result {
            ast::Statement::Assignment(assignment) => {
                assert_eq!(assignment.name, "x");
                match assignment.value {
                    ast::Expression::NumberLiteral(literal) => assert_eq!(literal.value, 42.0),
                    _ => panic!("Expected NumberLiteral as assignment value"),
                }
            }
            _ => panic!("Expected Assignment"),
        }
    }

    #[test]
    fn test_parse_return_statement() {
        let input = "return 42";
        let result = parse_statement_input(input, Rule::return_statement);

        match result {
            ast::Statement::Return(return_stmt) => match *return_stmt.value.unwrap() {
                ast::Expression::NumberLiteral(literal) => assert_eq!(literal.value, 42.0),
                _ => panic!("Expected NumberLiteral as return value"),
            },
            _ => panic!("Expected ReturnStatement"),
        }
    }

    #[test]
    fn test_parse_block_statement() {
        let input = r#"{
            x := 42
            x = 43
        }
        "#;
        let result = parse_statement_input(input, Rule::statement);

        match result {
            ast::Statement::Block(block) => {
                assert_eq!(block.statements.len(), 2, "{:?}", block.statements);

                // Check the first statement
                match &block.statements[0] {
                    ast::Statement::VariableDeclaration(var_decl) => {
                        match *var_decl.pattern {
                            ast::Pattern::Identifier(ast::IdentifierPattern { span })
                                if span.as_str() == "x" => {}
                            _ => panic!("Identifier pattern expected"),
                        };
                        match *var_decl.value.clone() {
                            ast::Expression::NumberLiteral(literal) => {
                                assert_eq!(literal.value, 42.0)
                            }
                            _ => panic!("Expected NumberLiteral as variable value"),
                        }
                    }
                    _ => panic!("Expected VariableDeclaration"),
                }

                // Check the second statement
                match &block.statements[1] {
                    ast::Statement::Assignment(assignment) => {
                        assert_eq!(assignment.name, "x");
                        match &assignment.value {
                            ast::Expression::NumberLiteral(literal) => {
                                assert_eq!(literal.value, 43.0)
                            }
                            _ => panic!("Expected NumberLiteral as assignment value"),
                        }
                    }
                    _ => panic!("Expected Assignment"),
                }
            }
            _ => panic!("Expected BlockStatement, got {:?}", result),
        }
    }

    #[test]
    fn test_parse_expression_statement() {
        let input = "42;";
        let result = parse_statement_input(input, Rule::expression_statement);

        match result {
            ast::Statement::Expression(expr_stmt) => match *expr_stmt.expression {
                ast::Expression::NumberLiteral(literal) => assert_eq!(literal.value, 42.0),
                _ => panic!("Expected NumberLiteral as expression"),
            },
            _ => panic!("Expected ExpressionStatement"),
        }
    }
}
