use pest::iterators::Pair;

use crate::ast::Node;

use super::parser::{ParseError, ParseResult, Rule};

pub fn parse_expression<'i>(pair: Pair<'i, Rule>) -> ParseResult<'i, Node> {
    match pair.as_rule() {
        Rule::expression | Rule::primary => match pair.into_inner().next() {
            Some(inner) => parse_expression(inner),
            // TODO:
            None => Err(vec![]),
        },
        Rule::equality
        | Rule::relation
        | Rule::addition
        | Rule::multiplication
        | Rule::exponentiation => parse_binary_expression(pair),
        Rule::identifier => Ok(Node::Identifier(pair.as_str().to_string())),
        Rule::string_literal => {
            let value = pair.as_str();
            Ok(Node::StringLiteral(value[1..value.len() - 1].to_string()))
        }
        Rule::number_literal => Ok(Node::NumberLiteral(pair.as_str().parse().unwrap_or(0.0))),
        Rule::boolean_literal => Ok(Node::BooleanLiteral(pair.as_str() == "true")),
        // TODO:
        _ => Err(vec![]),
    }
}

fn parse_binary_expression<'i>(pair: Pair<'i, Rule>) -> ParseResult<'i, Node> {
    let mut errors = Vec::new();
    let span = pair.clone().as_span();

    let mut inner = pair.into_inner();
    let Some(next) = inner.next() else {
        // FIXME: does not handle "** expression"
        return Err(vec![]);
    };
    let mut left = match parse_expression(next) {
        Ok(expr) => Some(expr),
        Err(mut err) => {
            errors.append(&mut err);
            None
        }
    };

    while let Some(op_pair) = inner.next() {
        let operator = op_pair.as_str().to_string();

        let Some(right_pair) = inner.next() else {
            errors.push(ParseError {
                message: "Expression expected".to_string(),
                span: op_pair.as_span(),
            });
            continue;
        };

        let right = match parse_expression(right_pair) {
            Ok(expr) => Some(expr),
            Err(mut e) => {
                errors.append(&mut e);
                None
            }
        };

        left = match (left, right) {
            (Some(l), Some(r)) => Some(Node::BinaryExpression {
                left: Box::new(l),
                operator,
                right: Box::new(r),
            }),
            _ => None,
        };
    }

    if errors.is_empty() {
        match left {
            Some(l) => Ok(l),
            None => Err(vec![]),
        }
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::MyLanguageParser;
    use crate::parser::parser::Rule;
    use pest::Parser;

    fn parse(input: &str) -> Node {
        let pair = MyLanguageParser::parse(Rule::expression, input)
            .unwrap()
            .next()
            .unwrap();
        parse_expression(pair).expect("Failed to parse expression")
    }

    #[test]
    fn test_parse_number_literal() {
        let node = parse("42");
        assert!(matches!(node, Node::NumberLiteral(42.0)));
    }

    #[test]
    fn test_parse_string_literal() {
        let node = parse("\"hello\"");
        assert!(matches!(node, Node::StringLiteral(s) if s == "hello"));
    }

    #[test]
    fn test_parse_boolean_literal() {
        assert!(matches!(parse("true"), Node::BooleanLiteral(true)));
        assert!(matches!(parse("false"), Node::BooleanLiteral(false)));
    }

    #[test]
    fn test_parse_identifier() {
        let node = parse("my_var");
        assert!(matches!(node, Node::Identifier(s) if s == "my_var"));
    }

    #[test]
    fn test_parse_simple_binary_expression() {
        let node = parse("1 + 2");
        match node {
            Node::BinaryExpression { operator, .. } => assert_eq!(operator, "+"),
            _ => panic!("Expected BinaryExpression"),
        }
    }

    #[test]
    fn test_parse_nested_binary_expression() {
        let node = parse("1 + 2 * 3");
        let right = match node {
            Node::BinaryExpression {
                operator, right, ..
            } => {
                assert_eq!(operator, "+");
                right
            }
            _ => panic!("Expected BinaryExpression"),
        };
        match *right {
            Node::BinaryExpression { operator, .. } => assert_eq!(operator, "*"),
            _ => panic!("Expected inner BinaryExpression"),
        }
    }

    #[test]
    fn test_parse_parenthesized_expression() {
        let node = parse("(1 + 2)");
        match node {
            Node::BinaryExpression { operator, .. } => assert_eq!(operator, "+"),
            _ => panic!("Expected BinaryExpression"),
        }
    }

    #[test]
    fn test_parse_function_call_no_args() {
        let node = parse("foo()");
        match node {
            Node::FunctionCall { name, args } => {
                assert_eq!(name, "foo");
                assert!(args.is_empty());
            }
            _ => panic!("Expected FunctionCall"),
        }
    }

    #[test]
    fn test_parse_function_call_with_args() {
        let node = parse("add(1, 2)");
        match node {
            Node::FunctionCall { name, args } => {
                assert_eq!(name, "add");
                assert_eq!(args.len(), 2);
                assert!(matches!(args[0], Node::NumberLiteral(1.0)));
                assert!(matches!(args[1], Node::NumberLiteral(2.0)));
            }
            _ => panic!("Expected FunctionCall"),
        }
    }
}
