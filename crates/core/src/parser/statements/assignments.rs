use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, ParserEngine},
};

impl ParserEngine {
    pub fn parse_assignment(&mut self, pair: Pair<'_, Rule>) -> ast::Assignment {
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();

        let pattern = self.parse_assignee(inner.next().unwrap());
        let op_loc = self.localize(inner.next().unwrap().as_span());
        let value = self.parse_expression(inner.next().unwrap());
        if value.is_empty() {
            self.error("expected expression".into(), op_loc.increment());
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
        parser::parser::{TineParser, Rule},
        ParseError,
    };
    use pest::Parser;

    fn parse_statement_input(input: &'static str, rule: Rule) -> (ast::Statement, Vec<ParseError>) {
        let pair = TineParser::parse(rule, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new(0);
        let stmt = parser_engine.parse_statement(pair);
        (stmt, parser_engine.errors)
    }

    #[test]
    fn test_parse_simple_assignment() {
        let input = "x = 42";
        let (stmt, errors) = parse_statement_input(input, Rule::assignment);

        assert_eq!(errors.len(), 0);
        let ast::Statement::Assignment(assignment) = stmt else {
            panic!("Expected Assignment")
        };

        match assignment.pattern {
            ast::Assignee::Pattern(ast::Pattern::Identifier(id)) if id.as_str() == "x" => {}
            _ => panic!("Expected 'x' as the assignee"),
        }

        match assignment.value {
            ast::Expression::NumberLiteral(literal) => assert_eq!(literal.value, 42.0),
            _ => panic!("Expected NumberLiteral as assignment value"),
        }
    }

    #[test]
    fn test_parse_simple_assignment_missing_value() {
        let input = "x = ";
        let (stmt, errors) = parse_statement_input(input, Rule::assignment);

        assert_eq!(errors.len(), 1);
        let ast::Statement::Assignment(assignment) = stmt else {
            panic!("Expected Assignment")
        };

        match assignment.pattern {
            ast::Assignee::Pattern(ast::Pattern::Identifier(id)) if id.as_str() == "x" => {}
            _ => panic!("Expected 'x' as the assignee"),
        }

        assert!(assignment.value.is_empty());
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
