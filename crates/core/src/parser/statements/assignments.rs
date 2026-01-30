use pest::iterators::Pair;

use crate::{
    ast,
    diagnostics::DiagnosticKind,
    parser::{parser::Rule, ParserEngine},
};

impl ParserEngine {
    pub fn parse_assignment(&mut self, pair: Pair<'_, Rule>) -> ast::Assignment {
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();

        let pattern = self.parse_assignee(inner.next().unwrap());
        let op_loc = self.localize(inner.next().unwrap().as_span());
        let value = inner
            .next()
            .map(|pair| self.parse_expression(pair))
            .unwrap_or(ast::Expression::Empty);
        if value.is_empty() {
            self.error(DiagnosticKind::MissingExpression, op_loc.increment());
        }

        ast::Assignment {
            loc,
            pattern,
            value,
        }
    }

    fn parse_assignee(&mut self, pair: Pair<'_, Rule>) -> ast::Assignee {
        assert!(pair.as_rule() == Rule::assignee);
        let pair = pair.into_inner().next().unwrap();
        match pair.as_rule() {
            Rule::indirection => self.parse_indirection_assignee(pair).into(),
            Rule::member_assignee => self.parse_member_assignee(pair).into(),
            Rule::pattern => self.parse_pattern(pair).into(),
            rule => unreachable!("Unexpected rule {:?}", rule),
        }
    }

    fn parse_member_assignee(&mut self, pair: Pair<'_, Rule>) -> ast::MemberExpression {
        debug_assert_eq!(pair.as_rule(), Rule::member_assignee);
        let mut inner = pair.into_inner();
        let root = self.parse_identifier(inner.next().unwrap());
        let mut node = self.parse_member_expression(root.into(), inner.next().unwrap());
        for suffix in inner {
            node = self.parse_member_expression(node.into(), suffix);
        }
        node
    }

    fn parse_indirection_assignee(&mut self, pair: Pair<'_, Rule>) -> ast::IndirectionAssignee {
        assert_eq!(pair.as_rule(), Rule::indirection);
        let loc = self.localize(pair.as_span());
        let identifier = self.parse_identifier(pair.into_inner().next().unwrap());
        ast::IndirectionAssignee { loc, identifier }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        diagnostics::Diagnostic,
        parser::parser::{Rule, TineParser},
        Location, Span,
    };
    use pest::Parser;

    fn parse_statement_input(input: &'static str, rule: Rule) -> (ast::Statement, Vec<Diagnostic>) {
        let pair = TineParser::parse(rule, input).unwrap().next().unwrap();
        let mut parser_engine = ParserEngine::new(0);
        let stmt = parser_engine.parse_statement(pair);
        (stmt, parser_engine.diagnostics)
    }

    #[test]
    fn test_parse_simple_assignment() {
        let input = "x = 42";
        let (stmt, errors) = parse_statement_input(input, Rule::assignment);

        let expected = ast::Statement::Assignment(ast::Assignment {
            loc: Location::new(0, Span::new(0, input.len() as u32)),
            pattern: ast::Assignee::Pattern(ast::Pattern::Identifier(ast::IdentifierPattern(
                ast::Identifier {
                    loc: Location::new(0, Span::new(0, 1)),
                    text: "x".to_string(),
                },
            ))),
            value: ast::Expression::IntLiteral(ast::IntLiteral {
                loc: Location::new(0, Span::new(4, 6)),
                value: 42,
            }),
        });

        assert_eq!(errors.len(), 0);
        assert_eq!(stmt, expected);
    }

    #[test]
    fn test_parse_simple_assignment_missing_value() {
        let input = "x = ";
        let expected = ast::Statement::Assignment(ast::Assignment {
            loc: Location::new(0, Span::new(0, input.len() as u32)),
            pattern: ast::Assignee::Pattern(ast::Pattern::Identifier(ast::IdentifierPattern(
                ast::Identifier {
                    loc: Location::new(0, Span::new(0, 1)),
                    text: "x".to_string(),
                },
            ))),
            value: ast::Expression::Empty,
        });

        let (stmt, errors) = parse_statement_input(input, Rule::assignment);

        assert_eq!(stmt, expected);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].kind, DiagnosticKind::MissingExpression));
    }

    #[test]
    fn test_parse_member_expression_assignment() {
        let input = "user.name = \"John\"";
        let (stmt, errors) = parse_statement_input(input, Rule::assignment);

        assert_eq!(errors.len(), 0);
        let ast::Statement::Assignment(assignment) = stmt else {
            panic!("Expected Assignment")
        };

        match assignment.pattern {
            ast::Assignee::Member(_) => {}
            _ => panic!("Expected FieldAccessExpression as the assignee"),
        }

        match assignment.value {
            ast::Expression::StringLiteral(literal) => assert_eq!(literal.as_str(), "John"),
            _ => panic!("Expected StringLiteral as assignment value"),
        }
    }

    #[test]
    fn test_parse_nested_member_assignment() {
        let input = "user.address.city = \"New York\"";
        let (stmt, errors) = parse_statement_input(input, Rule::assignment);

        assert_eq!(errors.len(), 0);
        let ast::Statement::Assignment(assignment) = stmt else {
            panic!("Expected Assignment")
        };

        match assignment.pattern {
            ast::Assignee::Member(_) => {}
            _ => panic!("Expected FieldAccessExpression as the assignee"),
        }

        match assignment.value {
            ast::Expression::StringLiteral(literal) => {
                assert_eq!(literal.as_str(), "New York")
            }
            _ => panic!("Expected StringLiteral as assignment value"),
        }
    }
}
