use pest::iterators::Pair;

use crate::ast::{Node, Spanned};

use super::{
    expressions::parse_expression,
    parser::{ParseResult, Rule},
    utils::merge_span,
};

pub fn parse_type(pair: Pair<'static, Rule>) -> ParseResult {
    match pair.as_rule() {
        Rule::type_annotation | Rule::type_name => match pair.into_inner().next() {
            Some(inner) => parse_type(inner),
            None => ParseResult::empty(),
        },
        Rule::function_type => parse_function_type(pair),
        Rule::tuple_type => parse_tuple_type(pair),
        Rule::binary_type => parse_binary_type(pair),
        Rule::unary_type => parse_unary_type(pair),
        Rule::generic_type => parse_generic_type(pair),
        Rule::identifier => parse_expression(pair),
        _ => unreachable!(),
    }
}

fn parse_tuple_type(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::tuple_type);
    let span = pair.as_span();
    let mut sub_types = Vec::new();
    let mut errors = Vec::new();
    for sub_pair in pair.into_inner() {
        let mut result = parse_type(sub_pair);
        sub_types.push(result.node);
        errors.append(&mut result.errors);
    }

    let node: Option<Spanned<Node>> = match sub_types.len() {
        0 => None,
        1 => sub_types.pop().unwrap(),
        _ => Some(Spanned {
            node: Node::Tuple(sub_types),
            span,
        }),
    };

    ParseResult { node, errors }
}

fn parse_binary_type(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::binary_type);
    let mut operands = Vec::new();
    let mut operators = Vec::new();
    let mut errors = Vec::new();

    let mut previous_was_operand = false;
    for part in pair.into_inner() {
        match part.as_rule() {
            Rule::binary_type_op => {
                if !previous_was_operand {
                    operands.push(None);
                }
                operators.push(part);
                previous_was_operand = false;
            }
            _ => {
                let mut result = parse_type(part);
                errors.append(&mut result.errors);
                operands.push(result.node);
                previous_was_operand = true;
            }
        }
    }

    // Handle missing last operand (e.g. T1#T2#)
    if operands.len() == operators.len() {
        operands.push(None);
    }

    let node = match operands.len() {
        0 => None,
        1 => operands.pop().unwrap(),
        _ => {
            let mut node = operands.pop().unwrap();
            while let Some(operand) = operands.pop() {
                let operator = operators.pop().unwrap();
                let span = match (operand.clone(), node.clone()) {
                    (Some(lhs), Some(rhs)) => merge_span(lhs.span, rhs.span),
                    (Some(lhs), None) => merge_span(lhs.span, operator.as_span()),
                    (None, Some(rhs)) => merge_span(operator.as_span(), rhs.span),
                    (None, None) => operator.as_span(),
                };
                node = Some(Spanned {
                    node: Node::BinaryType {
                        left: operand.map(Box::new),
                        operator: operator.as_str().to_string(),
                        right: node.map(Box::new),
                    },
                    span,
                })
            }
            node
        }
    };

    ParseResult { node, errors }
}

fn parse_unary_type(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::unary_type);
    let inner = pair.into_inner();

    let mut ops = vec![];
    let mut node = None;
    let mut errors = Vec::new();

    for part in inner {
        match part.as_rule() {
            Rule::unary_type_op => ops.push(part),
            Rule::generic_type | Rule::type_name => {
                let mut result = parse_type(part);
                errors.append(&mut result.errors);
                node = result.node;
            }
            _ => unreachable!(),
        }
    }

    for op in ops.into_iter().rev() {
        let span = match node {
            Some(ref n) => merge_span(op.as_span(), n.span),
            None => op.as_span(),
        };
        node = Some(Spanned {
            node: Node::UnaryType(node.map(Box::new)),
            span,
        })
    }

    ParseResult { node, errors }
}

fn parse_generic_type(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::generic_type);
    let span = pair.as_span();
    let mut name = None;
    let mut args = Vec::new();
    let mut errors = Vec::new();

    for part in pair.into_inner() {
        match part.as_rule() {
            Rule::identifier => {
                name = Some(part.as_str().to_string());
            }
            Rule::binary_type => {
                let mut result = parse_type(part);
                errors.append(&mut result.errors);
                args.push(Box::new(result.node.unwrap()));
            }
            rule => unreachable!("Unexpected rule '{:?}'", rule),
        }
    }

    let node = if args.len() > 0 {
        Node::GenericType {
            name: name.unwrap(),
            args,
        }
    } else {
        Node::Identifier(name.unwrap())
    };

    ParseResult {
        node: Some(Spanned { node, span }),
        errors,
    }
}

fn parse_function_type(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::function_type);
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let param_type = inner
        .next()
        .map(parse_type)
        .unwrap_or_else(ParseResult::empty);

    let return_type = inner
        .next()
        .map(parse_type)
        .unwrap_or_else(ParseResult::empty);

    let mut errors = param_type.errors;
    errors.extend(return_type.errors);

    ParseResult {
        node: Some(Spanned {
            node: Node::FunctionType {
                parameters: Box::new(param_type.node.unwrap()),
                return_type: Box::new(return_type.node.unwrap()),
            },
            span,
        }),
        errors,
    }
}
#[cfg(test)]
mod tests {
    use pest::Parser;

    use crate::parser::parser::MyLanguageParser;

    use super::*;

    fn parse_type_input(input: &'static str) -> ParseResult {
        let pair = MyLanguageParser::parse(Rule::type_annotation, input)
            .unwrap()
            .next()
            .unwrap();
        parse_type(pair)
    }

    #[test]
    fn test_parse_single_tuple_type() {
        let result = parse_type_input("number");
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::Identifier(name) => assert_eq!(name, "number"),
            _ => panic!("Expected Identifier"),
        }
    }

    #[test]
    fn test_parse_multi_tuple_type() {
        let result = parse_type_input("number, string");
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::Tuple(items) => {
                assert_eq!(items.len(), 2);
                assert_eq!(
                    items[0].as_ref().unwrap().node,
                    Node::Identifier("number".into())
                );
                assert_eq!(
                    items[1].as_ref().unwrap().node,
                    Node::Identifier("string".into())
                );
            }
            _ => panic!("Expected Tuple node"),
        }
    }

    #[test]
    fn test_parse_binary_type() {
        let result = parse_type_input("A#B!C");
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::BinaryType { .. } => {} // You can deeply pattern-match here if needed
            _ => panic!("Expected BinaryType"),
        }
    }

    #[test]
    fn test_parse_unary_type() {
        let result = parse_type_input("?[]Foo");
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::UnaryType(inner1) => match inner1.unwrap().node {
                Node::UnaryType(inner2) => match inner2.unwrap().node {
                    Node::Identifier(ref name) => assert_eq!(name, "Foo"),
                    _ => panic!("Expected inner identifier"),
                },
                _ => panic!("Expected nested unary"),
            },
            _ => panic!("Expected outer unary"),
        }
    }

    #[test]
    fn test_parse_generic_type_no_args() {
        let result = parse_type_input("Box");
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::Identifier(name) => assert_eq!(name, "Box"),
            _ => panic!("Expected Identifier (generic type without args)"),
        }
    }

    #[test]
    fn test_parse_generic_type_with_args() {
        let result = parse_type_input("List[number, string]");
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::GenericType { name, args } => {
                assert_eq!(name, "List");
                assert_eq!(args.len(), 2);
                assert_eq!(args[0].node, Node::Identifier("number".into()));
                assert_eq!(args[1].node, Node::Identifier("string".into()));
            }
            _ => panic!("Expected GenericType"),
        }
    }

    #[test]
    fn test_parse_function_type() {
        let result = parse_type_input("(number, string) -> boolean");
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::FunctionType {
                parameters,
                return_type,
            } => {
                match parameters.node {
                    Node::Tuple(params) => {
                        assert_eq!(params.len(), 2);
                        assert_eq!(
                            params[0].as_ref().unwrap().node,
                            Node::Identifier("number".into())
                        );
                        assert_eq!(
                            params[1].as_ref().unwrap().node,
                            Node::Identifier("string".into())
                        );
                    }
                    _ => panic!("Expected parameters as Tuple"),
                }
                assert_eq!(return_type.node, Node::Identifier("boolean".into()));
            }
            _ => panic!("Expected FunctionType"),
        }
    }
}
