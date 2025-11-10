mod type_aliases;

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
            Rule::break_statement => self.parse_break_statement(pair).into(),
            Rule::method_definition => self.parse_method_definition(pair).into(),
            Rule::return_statement => self.parse_return_statement(pair).into(),
            Rule::expression_statement => self.parse_expression_statement(pair),
            _ => ast::Statement::Empty,
        }
    }

    pub fn parse_variable_declaration(
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

        let pattern = self.parse_assignee(inner.next().unwrap());
        let value = self.parse_expression(inner.next().unwrap());

        ast::Assignment {
            span,
            pattern,
            value,
        }
    }

    fn parse_assignee(&mut self, pair: Pair<'static, Rule>) -> ast::Assignee {
        assert!(pair.as_rule() == Rule::assignee);
        let pair = pair.into_inner().next().unwrap();
        match pair.as_rule() {
            Rule::indirection => self.parse_indirection_assignee(pair).into(),
            Rule::member_assignee => self.parse_member_assignee(pair).into(),
            Rule::pattern => self.parse_pattern(pair).into(),
            rule => unreachable!("Unexpected rule {:?}", rule),
        }
    }

    fn parse_member_assignee(&mut self, pair: Pair<'static, Rule>) -> ast::MemberExpression {
        debug_assert_eq!(pair.as_rule(), Rule::member_assignee);
        let mut inner = pair.into_inner();
        let root = self.parse_identifier(inner.next().unwrap());
        let mut node = self.parse_member_expression(root.into(), inner.next().unwrap());
        for suffix in inner {
            node = self.parse_member_expression(node.into(), suffix);
        }
        node
    }

    fn parse_indirection_assignee(
        &mut self,
        pair: Pair<'static, Rule>,
    ) -> ast::IndirectionAssignee {
        assert_eq!(pair.as_rule(), Rule::indirection);
        let span = pair.as_span();
        let identifier = self.parse_identifier(pair.into_inner().next().unwrap());
        ast::IndirectionAssignee { span, identifier }
    }

    fn parse_break_statement(&mut self, pair: Pair<'static, Rule>) -> ast::BreakStatement {
        assert_eq!(pair.as_rule(), Rule::break_statement);
        let span = pair.as_span();
        let value = pair
            .into_inner()
            .next()
            .map(|inner| self.parse_expression(inner))
            .map(Box::new);

        ast::BreakStatement { span, value }
    }

    fn parse_method_definition(&mut self, pair: Pair<'static, Rule>) -> ast::MethodDefinition {
        assert_eq!(pair.as_rule(), Rule::method_definition);
        let span = pair.as_span();
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

    fn parse_method_receiver(&mut self, pair: Pair<'static, Rule>) -> ast::MethodReceiver {
        assert_eq!(pair.as_rule(), Rule::method_receiver);
        let span = pair.as_span();
        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_span().into();
        let ty = self.parse_named_type(inner.next().unwrap());
        ast::MethodReceiver { span, name, ty }
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
    fn test_parse_simple_assignment() {
        let input = "x = 42";
        let result = parse_statement_input(input, Rule::assignment);

        match result {
            ast::Statement::Assignment(assignment) => {
                // Check the pattern
                match assignment.pattern {
                    ast::Assignee::Pattern(ast::Pattern::Identifier(id))
                        if id.span.as_str() == "x" => {}
                    _ => panic!("Expected 'x' as the assignee"),
                }

                // Check the value
                match assignment.value {
                    ast::Expression::NumberLiteral(literal) => assert_eq!(literal.value, 42.0),
                    _ => panic!("Expected NumberLiteral as assignment value"),
                }
            }
            _ => panic!("Expected Assignment"),
        }
    }

    #[test]
    fn test_parse_member_expression_assignment() {
        let input = "user.name = \"John\"";
        let result = parse_statement_input(input, Rule::assignment);

        match result {
            ast::Statement::Assignment(assignment) => {
                // Check the pattern
                match assignment.pattern {
                    ast::Assignee::Member(expr) => {
                        assert_eq!(expr.object.as_span().as_str(), "user");
                        assert!(matches!(expr.prop, Some(ast::MemberProp::FieldName(_))));
                        assert_eq!(expr.prop.unwrap().as_span().as_str(), "name");
                    }
                    _ => panic!("Expected FieldAccessExpression as the assignee"),
                }

                // Check the value
                match assignment.value {
                    ast::Expression::StringLiteral(literal) => assert_eq!(literal.as_str(), "John"),
                    _ => panic!("Expected StringLiteral as assignment value"),
                }
            }
            _ => panic!("Expected Assignment"),
        }
    }

    #[test]
    fn test_parse_nested_assignment() {
        let input = "user.address.city = \"New York\"";
        let result = parse_statement_input(input, Rule::assignment);

        match result {
            ast::Statement::Assignment(assignment) => {
                // Check the pattern
                match assignment.pattern {
                    ast::Assignee::Member(expr) => {
                        assert_eq!(expr.object.as_span().as_str(), "user.address");
                        assert!(matches!(expr.prop, Some(ast::MemberProp::FieldName(_))));
                        assert_eq!(expr.prop.unwrap().as_span().as_str(), "city");
                    }
                    _ => panic!("Expected FieldAccessExpression as the assignee"),
                }

                // Check the value
                match assignment.value {
                    ast::Expression::StringLiteral(literal) => {
                        assert_eq!(literal.as_str(), "New York")
                    }
                    _ => panic!("Expected StringLiteral as assignment value"),
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
