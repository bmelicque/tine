mod assignments;
mod functions;
mod type_definitions;
mod variable_declarations;

use pest::iterators::Pair;

use crate::ast;

use super::{parser::Rule, ParserEngine};

impl ParserEngine {
    pub fn parse_statement(&mut self, pair: Pair<'_, Rule>) -> ast::Statement {
        match pair.as_rule() {
            Rule::statement => {
                let inner_pair = pair.into_inner().next().unwrap();
                self.parse_statement(inner_pair)
            }
            Rule::variable_declaration => self.parse_variable_declaration(pair).into(),
            Rule::assignment => self.parse_assignment(pair).into(),
            Rule::enum_definition => self.parse_enum_definition(pair).into(),
            Rule::function_definition => self.parse_function_definition(pair).into(),
            Rule::struct_definition => self.parse_struct_definition(pair).into(),
            Rule::type_alias => self.parse_type_alias(pair).into(),
            Rule::break_statement => self.parse_break_statement(pair).into(),
            Rule::method_definition => self.parse_method_definition(pair).into(),
            Rule::return_statement => self.parse_return_statement(pair).into(),
            Rule::expression_statement => self.parse_expression_statement(pair),
            _ => ast::Statement::Empty,
        }
    }

    fn parse_break_statement(&mut self, pair: Pair<'_, Rule>) -> ast::BreakStatement {
        assert_eq!(pair.as_rule(), Rule::break_statement);
        let loc = self.localize(pair.as_span());
        let value = pair
            .into_inner()
            .next()
            .map(|inner| self.parse_expression(inner))
            .map(Box::new);

        ast::BreakStatement { loc, value }
    }

    fn parse_method_definition(&mut self, pair: Pair<'_, Rule>) -> ast::MethodDefinition {
        assert_eq!(pair.as_rule(), Rule::method_definition);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let receiver = self.parse_method_receiver(inner.next().unwrap());
        let next = inner.next().unwrap();
        let name = ast::Identifier {
            loc: self.localize(next.as_span()),
            text: next.as_str().to_string(),
        };
        let definition = self.parse_function_expression(inner.next().unwrap());
        ast::MethodDefinition {
            loc,
            receiver,
            name,
            definition,
        }
    }

    fn parse_method_receiver(&mut self, pair: Pair<'_, Rule>) -> ast::MethodReceiver {
        assert_eq!(pair.as_rule(), Rule::method_receiver);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let next = inner.next().unwrap();
        let name = ast::Identifier {
            loc: self.localize(next.as_span()),
            text: next.as_str().to_string(),
        };
        let ty = self.parse_named_type(inner.next().unwrap());
        ast::MethodReceiver { loc, name, ty }
    }

    fn parse_return_statement(&mut self, pair: Pair<'_, Rule>) -> ast::ReturnStatement {
        let loc = self.localize(pair.as_span());
        let value = pair
            .into_inner()
            .next()
            .map(|inner| self.parse_expression(inner))
            .map(Box::new);

        ast::ReturnStatement { loc, value }
    }

    fn parse_expression_statement(&mut self, pair: Pair<'_, Rule>) -> ast::Statement {
        let mut expression = ast::Expression::Empty;
        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::comment => {}
                Rule::expression => expression = self.parse_expression(pair),
                rule => unreachable!("unexpected rule '{:?}'", rule),
            }
        }
        match expression {
            ast::Expression::Empty => ast::Statement::Empty,
            expression => ast::Statement::Expression(ast::ExpressionStatement {
                expression: Box::new(expression),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{Rule, TineParser};
    use pest::Parser;

    fn parse_statement_input(input: &'static str, rule: Rule) -> ast::Statement {
        let pair = TineParser::parse(rule, input).unwrap().next().unwrap();
        let mut parser_engine = ParserEngine::new(0);
        parser_engine.parse_statement(pair)
    }

    #[test]
    fn test_parse_return_statement() {
        let input = "return 42";
        let result = parse_statement_input(input, Rule::return_statement);

        match result {
            ast::Statement::Return(return_stmt) => match *return_stmt.value.unwrap() {
                ast::Expression::IntLiteral(literal) => assert_eq!(literal.value, 42),
                _ => panic!("Expected IntLiteral as return value"),
            },
            _ => panic!("Expected ReturnStatement"),
        }
    }

    #[test]
    fn test_parse_expression_statement() {
        let input = "42";
        let result = parse_statement_input(input, Rule::expression_statement);

        let ast::Statement::Expression(expr_stmt) = result else {
            panic!("Expected ExpressionStatement");
        };

        match *expr_stmt.expression {
            ast::Expression::IntLiteral(literal) => assert_eq!(literal.value, 42),
            _ => panic!("Expected IntLiteral as expression"),
        }
    }

    #[test]
    fn test_parse_expression_statement_with_comment() {
        let input = r#"// useless comment
42"#;
        let result = parse_statement_input(input, Rule::expression_statement);

        let ast::Statement::Expression(expr_stmt) = result else {
            panic!("Expected ExpressionStatement");
        };

        match *expr_stmt.expression {
            ast::Expression::IntLiteral(literal) => assert_eq!(literal.value, 42),
            _ => panic!("Expected IntLiteral as expression"),
        }
    }
}
