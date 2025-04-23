use pest::iterators::Pair;

use crate::ast::{AstNode, Node, Spanned};

use super::{
    expressions::parse_expression,
    parser::{ParseError, ParseResult, Rule},
};

pub fn parse_type(pair: Pair<'static, Rule>) -> ParseResult {
    match pair.as_rule() {
        Rule::type_annotation
        | Rule::type_element
        | Rule::binary_type
        | Rule::unary_type
        | Rule::primary_type
        | Rule::grouped_type => parse_type(pair.into_inner().next().unwrap()),
        Rule::function_type => parse_function_type(pair),
        Rule::tuple_type => parse_tuple_type(pair),
        Rule::map_type => parse_map_type(pair),
        Rule::result_type => parse_result_type(pair),
        Rule::reference_type => parse_reference_type(pair),
        Rule::option_type => parse_option_type(pair),
        Rule::array_type => parse_array_type(pair),
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

    let node = Some(Spanned {
        node: Node::TupleType(sub_types),
        span,
    });

    ParseResult { node, errors }
}

fn parse_map_type(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::map_type);
    let span = pair.as_span();
    let mut key = None;
    let mut value = None;
    let mut errors = Vec::new();

    for sub_pair in pair.into_inner() {
        match sub_pair.as_rule() {
            Rule::map_type_key => {
                let mut result = parse_type(sub_pair);
                key = result.node.map(Box::new);
                errors.append(&mut result.errors);
            }
            Rule::map_type_value => {
                let mut result = parse_type(sub_pair);
                value = result.node.map(Box::new);
                errors.append(&mut result.errors);
            }
            _ => unreachable!(),
        }
    }

    let node = Some(Spanned {
        node: Node::MapType { key, value },
        span,
    });

    ParseResult { node, errors }
}

fn parse_result_type(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::map_type);
    let span = pair.as_span();
    let mut ok = None;
    let mut err = None;
    let mut errors = Vec::new();

    for sub_pair in pair.into_inner() {
        match sub_pair.as_rule() {
            Rule::result_ok_type => {
                let mut result = parse_type(sub_pair);
                ok = result.node.map(Box::new);
                errors.append(&mut result.errors);
            }
            Rule::result_error_type => {
                let mut result = parse_type(sub_pair);
                err = result.node.map(Box::new);
                errors.append(&mut result.errors);
            }
            _ => unreachable!(),
        }
    }

    let node = Some(Spanned {
        node: Node::ResultType { ok, err },
        span,
    });

    ParseResult { node, errors }
}

fn parse_reference_type(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::reference_type);
    let span = pair.as_span();
    let inner_type = pair
        .into_inner()
        .next()
        .map(parse_type)
        .unwrap_or_else(ParseResult::empty);

    let errors = inner_type.errors;
    let node = Some(Spanned {
        node: Node::ReferenceType(inner_type.node.map(Box::new)),
        span,
    });

    ParseResult { node, errors }
}

fn parse_option_type(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::option_type);
    let span = pair.as_span();
    let inner_type = pair
        .into_inner()
        .next()
        .map(parse_type)
        .unwrap_or_else(ParseResult::empty);

    let errors = inner_type.errors;
    let node = Some(Spanned {
        node: Node::OptionType(inner_type.node.map(Box::new)),
        span,
    });

    ParseResult { node, errors }
}

fn parse_array_type(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::array_type);
    let span = pair.as_span();
    let inner_type = pair
        .into_inner()
        .next()
        .map(parse_type)
        .unwrap_or_else(ParseResult::empty);

    let errors = inner_type.errors;
    let node = Some(Spanned {
        node: Node::ArrayType(inner_type.node.map(Box::new)),
        span,
    });

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
                name = Some(Box::new(Spanned {
                    node: Node::NamedType(part.as_str().to_string()),
                    span: part.as_span(),
                }));
            }
            Rule::type_element => {
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
        name.unwrap().node
    };

    ParseResult {
        node: Some(Spanned { node, span }),
        errors,
    }
}

fn parse_function_type(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::function_type);
    let span = pair.as_span();

    let mut errors = Vec::new();

    let mut inner = pair.into_inner();
    let param_result = parse_function_type_params(inner.next().unwrap());
    let parameters = param_result
        .0
        .into_iter()
        .map(|param| Box::new(param))
        .collect();
    errors.extend(param_result.1);

    let returned_result = parse_type(inner.next().unwrap());
    let return_type = Box::new(returned_result.node.unwrap());
    errors.extend(returned_result.errors);

    ParseResult {
        node: Some(Spanned {
            node: Node::FunctionType {
                parameters,
                return_type,
            },
            span,
        }),
        errors,
    }
}

fn parse_function_type_params(pair: Pair<'static, Rule>) -> (Vec<AstNode>, Vec<ParseError>) {
    assert!(pair.as_rule() == Rule::function_type_params);
    let mut sub_types = Vec::new();
    let mut errors = Vec::new();
    for sub_pair in pair.into_inner() {
        let mut result = parse_type(sub_pair);
        if let Some(node) = result.node {
            sub_types.push(node);
        }
        errors.append(&mut result.errors);
    }

    (sub_types, errors)
}

#[cfg(test)]
mod tests {
    use pest::Parser;

    use super::*;
    use crate::parser::parser::MyLanguageParser;

    fn parse_type_input(input: &'static str) -> ParseResult {
        let pair = MyLanguageParser::parse(Rule::type_annotation, input)
            .unwrap()
            .next()
            .unwrap();
        parse_type(pair)
    }

    #[test]
    fn test_parse_named_type() {
        let result = parse_type_input("number");
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::NamedType(name) => assert_eq!(name, "number"),
            _ => panic!("Expected NamedType"),
        }
    }

    #[test]
    fn test_parse_tuple_type() {
        let result = parse_type_input("(number, string)");
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::TupleType(items) => {
                assert_eq!(items.len(), 2);
                assert_eq!(
                    items[0].as_ref().unwrap().node,
                    Node::NamedType("number".into())
                );
                assert_eq!(
                    items[1].as_ref().unwrap().node,
                    Node::NamedType("string".into())
                );
            }
            _ => panic!("Expected TupleType"),
        }
    }

    #[test]
    fn test_parse_generic_type_no_args() {
        let result = parse_type_input("Box");
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::NamedType(name) => assert_eq!(name, "Box"),
            _ => panic!("Expected NamedType (generic type without args)"),
        }
    }

    #[test]
    fn test_parse_generic_type_with_args() {
        let result = parse_type_input("List[number, string]");
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::GenericType { name, args } => {
                assert_eq!(name.node, Node::NamedType("List".into()));
                assert_eq!(args.len(), 2);
                assert_eq!(args[0].node, Node::NamedType("number".into()));
                assert_eq!(args[1].node, Node::NamedType("string".into()));
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
                assert_eq!(parameters.len(), 2);
                assert_eq!(
                    parameters[0].as_ref().node,
                    Node::NamedType("number".into())
                );
                assert_eq!(
                    parameters[1].as_ref().node,
                    Node::NamedType("string".into())
                );
                assert_eq!(return_type.node, Node::NamedType("boolean".into()));
            }
            _ => panic!("Expected FunctionType"),
        }
    }

    #[test]
    fn test_parse_array_type() {
        let result = parse_type_input("[number]");
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::ArrayType(inner) => {
                assert_eq!(inner.unwrap().node, Node::NamedType("number".into()));
            }
            _ => panic!("Expected ArrayType"),
        }
    }

    #[test]
    fn test_parse_option_type() {
        let result = parse_type_input("?number");
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::OptionType(inner) => {
                assert_eq!(inner.unwrap().node, Node::NamedType("number".into()));
            }
            _ => panic!("Expected OptionType"),
        }
    }

    #[test]
    fn test_parse_map_type() {
        let result = parse_type_input("{key: string, value: number}");
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::MapType { key, value } => {
                assert_eq!(key.unwrap().node, Node::NamedType("string".into()));
                assert_eq!(value.unwrap().node, Node::NamedType("number".into()));
            }
            _ => panic!("Expected MapType"),
        }
    }

    #[test]
    fn test_parse_result_type() {
        let result = parse_type_input("Result[number, string]");
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::ResultType { ok, err } => {
                assert_eq!(ok.unwrap().node, Node::NamedType("number".into()));
                assert_eq!(err.unwrap().node, Node::NamedType("string".into()));
            }
            _ => panic!("Expected ResultType"),
        }
    }
}
