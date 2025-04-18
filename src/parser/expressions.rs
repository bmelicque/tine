use pest::iterators::Pair;

use crate::ast::{Node, Spanned};

use super::{
    parser::{ParseError, ParseResult, Rule},
    type_instantiations::parse_type_instantiation,
};

pub fn parse_expression(pair: Pair<'static, Rule>) -> ParseResult {
    let span = pair.as_span().clone();

    match pair.as_rule() {
        Rule::expression | Rule::primary | Rule::type_annotation => {
            match pair.into_inner().next() {
                Some(inner) => parse_expression(inner),
                None => ParseResult {
                    node: None,
                    errors: vec![],
                },
            }
        }
        Rule::type_instantiation => parse_type_instantiation(pair),
        Rule::equality | Rule::relation | Rule::addition | Rule::multiplication => {
            parse_binary_ltr_expression(pair)
        }
        Rule::exponentiation => parse_binary_ltr_expression(pair),
        Rule::identifier => ParseResult {
            node: Some(Spanned {
                node: Node::Identifier(pair.as_str().to_string()),
                span,
            }),
            errors: vec![],
        },
        Rule::string_literal => {
            let value = pair.as_str();
            ParseResult {
                node: Some(Spanned {
                    node: Node::StringLiteral(value[1..value.len() - 1].to_string()),
                    span,
                }),
                errors: vec![],
            }
        }
        Rule::number_literal => ParseResult {
            node: Some(Spanned {
                node: Node::NumberLiteral(pair.as_str().parse().unwrap_or(0.0)),
                span,
            }),
            errors: vec![],
        },
        Rule::boolean_literal => ParseResult {
            node: Some(Spanned {
                node: Node::BooleanLiteral(pair.as_str() == "true"),
                span,
            }),
            errors: vec![],
        },
        _ => ParseResult {
            node: None,
            errors: vec![],
        },
    }
}

fn parse_binary_ltr_expression(pair: Pair<'static, Rule>) -> ParseResult {
    let span = pair.as_span().to_owned();
    let mut inner = pair.into_inner();
    let Some(next) = inner.next() else {
        return ParseResult::empty();
    };
    let result = parse_expression(next);
    let mut left = result.node;
    let mut errors = result.errors;

    let mut is_binary = false;
    while let Some(op_pair) = inner.next() {
        if !is_binary && left.is_none() && errors.is_empty() {
            errors.push(ParseError {
                message: "Expression expected".to_string(),
                span: op_pair.as_span(),
            });
        }
        is_binary = true;
        let operator = op_pair.as_str().to_string();

        let Some(right_pair) = inner.next() else {
            errors.push(ParseError {
                message: "Expression expected".to_string(),
                span: op_pair.as_span(),
            });
            continue;
        };

        let mut result = parse_expression(right_pair);
        let right = result.node;
        if right.is_none() && result.errors.is_empty() {
            errors.push(ParseError {
                message: "Expression expected".to_string(),
                span: op_pair.as_span(),
            });
        }
        errors.append(&mut result.errors);

        left = Some(Spanned {
            node: Node::BinaryExpression {
                left: left.map(Box::new),
                operator,
                right: right.map(Box::new),
            },
            // FIXME:
            span,
        });
    }

    ParseResult { node: left, errors }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{MyLanguageParser, Rule};
    use pest::Parser;

    fn parse(input: &'static str) -> ParseResult {
        let pair = MyLanguageParser::parse(Rule::expression, input)
            .unwrap()
            .next()
            .unwrap();
        parse_expression(pair)
    }

    #[test]
    fn test_parse_number_literal() {
        let result = parse("42");
        assert!(result.errors.is_empty());
        assert_eq!(result.node.unwrap().node, Node::NumberLiteral(42.0));
    }

    #[test]
    fn test_parse_string_literal() {
        let result = parse("\"hello\"");
        assert!(result.errors.is_empty());
        assert_eq!(
            result.node.unwrap().node,
            Node::StringLiteral("hello".to_string())
        );
    }

    #[test]
    fn test_parse_boolean_literal_true() {
        let result = parse("true");
        assert!(result.errors.is_empty());
        assert_eq!(result.node.unwrap().node, Node::BooleanLiteral(true));
    }

    #[test]
    fn test_parse_identifier() {
        let result = parse("foo");
        assert!(result.errors.is_empty());
        assert_eq!(
            result.node.unwrap().node,
            Node::Identifier("foo".to_string())
        );
    }

    #[test]
    fn test_simple_addition() {
        let result = parse("1 + 2");
        assert!(result.errors.is_empty());
        match result.node.unwrap().node {
            Node::BinaryExpression {
                left: Some(left),
                operator,
                right: Some(right),
            } => {
                assert_eq!(left.node, Node::NumberLiteral(1.0));
                assert_eq!(operator, "+");
                assert_eq!(right.node, Node::NumberLiteral(2.0));
            }
            _ => panic!("Expected binary expression with 1 + 2"),
        }
    }

    #[test]
    fn test_nested_binary_expression_ltr() {
        let result = parse("1 + 2 + 3");
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::BinaryExpression {
                operator,
                left: Some(left),
                right: Some(right),
            } => {
                assert_eq!(operator, "+");

                match left.node {
                    Node::BinaryExpression {
                        operator: ref inner_op,
                        left: Some(ref inner_left),
                        right: Some(ref inner_right),
                    } => {
                        assert_eq!(inner_op, "+");
                        assert_eq!(inner_left.node, Node::NumberLiteral(1.0));
                        assert_eq!(inner_right.node, Node::NumberLiteral(2.0));
                    }
                    _ => panic!("Expected nested binary expression on the left"),
                }

                assert_eq!(right.node, Node::NumberLiteral(3.0));
            }
            _ => panic!("Expected left-associative binary expression"),
        }
    }

    #[test]
    fn test_missing_rhs() {
        let result = parse("1 +");
        assert!(result.node.is_some());
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].message, "Expression expected");
    }

    #[test]
    fn test_missing_lhs() {
        let result = parse("+ 1");
        assert!(result.node.is_some());
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].message, "Expression expected");
    }

    #[test]
    fn test_only_operator() {
        let result = parse("+");
        assert!(result.node.is_some());
        assert_eq!(result.errors.len(), 2);
        assert_eq!(result.errors[0].message, "Expression expected");
        assert_eq!(result.errors[1].message, "Expression expected");
    }
}
