mod assignments;
mod type_aliases;
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
        let span = pair.as_span().into();
        let value = pair
            .into_inner()
            .next()
            .map(|inner| self.parse_expression(inner))
            .map(Box::new);

        ast::BreakStatement { span, value }
    }

    fn parse_method_definition(&mut self, pair: Pair<'_, Rule>) -> ast::MethodDefinition {
        assert_eq!(pair.as_rule(), Rule::method_definition);
        let span = pair.as_span().into();
        let mut inner = pair.into_inner();
        let receiver = self.parse_method_receiver(inner.next().unwrap());
        let name = inner.next().unwrap().as_span().into();
        let definition = self.parse_function_expression(inner.next().unwrap());
        ast::MethodDefinition {
            span,
            receiver,
            name,
            definition,
        }
    }

    fn parse_method_receiver(&mut self, pair: Pair<'_, Rule>) -> ast::MethodReceiver {
        assert_eq!(pair.as_rule(), Rule::method_receiver);
        let span = pair.as_span().into();
        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_span().into();
        let ty = self.parse_named_type(inner.next().unwrap());
        ast::MethodReceiver { span, name, ty }
    }

    fn parse_return_statement(&mut self, pair: Pair<'_, Rule>) -> ast::ReturnStatement {
        let span = pair.as_span().into();
        let value = pair
            .into_inner()
            .next()
            .map(|inner| self.parse_expression(inner))
            .map(Box::new);

        ast::ReturnStatement { span, value }
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
    fn test_parse_expression_statement() {
        let input = "42";
        let result = parse_statement_input(input, Rule::expression_statement);

        let ast::Statement::Expression(expr_stmt) = result else {
            panic!("Expected ExpressionStatement");
        };

        match *expr_stmt.expression {
            ast::Expression::NumberLiteral(literal) => assert_eq!(literal.value, 42.0),
            _ => panic!("Expected NumberLiteral as expression"),
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
            ast::Expression::NumberLiteral(literal) => assert_eq!(literal.value, 42.0),
            _ => panic!("Expected NumberLiteral as expression"),
        }
    }
}
