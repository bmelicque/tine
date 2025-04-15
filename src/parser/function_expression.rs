use pest::iterators::Pair;

use crate::ast::{Node, Parameter, Spanned};

use super::{
    expressions::{parse_expression, parse_type_annotation},
    parser::{ParseError, ParseResult, Rule},
    statements::parse_block,
};

pub fn parse_function_expression(pair: Pair<'static, Rule>) -> ParseResult {
    let mut errors = Vec::new();
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let mut parameters = None;
    let mut return_type = None;
    let mut body = None;

    while let Some(item) = inner.next() {
        match item.as_rule() {
            Rule::parameter_list => {
                parameters = Some(parse_parameter_list(item));
            }
            Rule::function_body => {
                let mut inner_body = item.into_inner();
                let Some(mut item) = inner_body.next() else {
                    panic!("function_body should at least contain a block!");
                };
                if item.as_rule() == Rule::type_annotation {
                    return_type = Some(parse_type_annotation(item));
                    item = if let Some(inner) = inner_body.next() {
                        inner
                    } else {
                        panic!("function_body should at contain a block!");
                    };
                } else {
                    errors.push(ParseError {
                        message: "Return type expected".to_string(),
                        span,
                    });
                }
                let mut body_result = parse_block(item);
                if !body_result.errors.is_empty() {
                    errors.append(&mut body_result.errors);
                }
                body = body_result.node.map(Box::new);
            }
            Rule::expression => {
                let mut result = parse_expression(item);
                if !result.errors.is_empty() {
                    errors.append(&mut result.errors);
                }
                body = result.node.map(Box::new);
            }
            _ => {}
        }
    }

    if let Some(ref params) = parameters {
        for parameter in params {
            if parameter.type_annotation.is_none() {
                errors.push(ParseError {
                    message: "Missing type anotation".to_string(),
                    // FIXME: span should be only parameter's span
                    span,
                });
            }
        }
    } else {
        errors.push(ParseError {
            message: "Parameters expected".to_string(),
            span,
        });
    }

    if body.is_none() {
        errors.push(ParseError {
            message: "Body expected".to_string(),
            span,
        });
    }

    ParseResult {
        node: Some(Spanned {
            node: Node::FunctionExpression {
                parameters,
                return_type,
                body,
            },
            span,
        }),
        errors,
    }
}

fn parse_parameter_list(pair: Pair<'static, Rule>) -> Vec<Parameter> {
    pair.into_inner()
        .filter(|p| p.as_rule() == Rule::parameter)
        .map(parse_parameter)
        .collect()
}

fn parse_parameter(pair: Pair<'static, Rule>) -> Parameter {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let type_annotation = inner.next().map(parse_type_annotation);

    Parameter {
        name,
        type_annotation,
    }
}

#[cfg(test)]
mod tests {
    use pest::Parser;

    use crate::parser::parser::MyLanguageParser;

    use super::*;

    fn parse_expr(input: &'static str) -> ParseResult {
        let pair = MyLanguageParser::parse(Rule::expression, input)
            .unwrap()
            .next()
            .unwrap();
        parse_expression(pair)
    }

    #[test]
    fn parses_function_with_no_params_and_expression_body() {
        let result = parse_expr("=> 42");
        assert!(!result.errors.is_empty());

        if let Some(Spanned {
            node:
                Node::FunctionExpression {
                    parameters,
                    return_type,
                    body,
                },
            ..
        }) = result.node
        {
            assert_eq!(parameters, None);
            assert!(return_type.is_none());
            assert!(matches!(body.unwrap().node, Node::NumberLiteral(_)));
        } else {
            panic!("Expected a FunctionExpression node");
        }
    }

    #[test]
    fn parses_function_with_params_and_expression_body() {
        let result = parse_expr("(x, y) => x + y");
        assert!(!result.errors.is_empty());

        if let Some(Spanned {
            node:
                Node::FunctionExpression {
                    parameters,
                    return_type,
                    body,
                },
            ..
        }) = result.node
        {
            let params = parameters.unwrap();
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].name, "x");
            assert_eq!(params[1].name, "y");
            assert!(return_type.is_none());
            assert!(matches!(body.unwrap().node, Node::BinaryExpression { .. }));
        } else {
            panic!("Expected a FunctionExpression node");
        }
    }

    #[test]
    fn parses_function_with_typed_params_and_block_body() {
        let result = parse_expr("(x number) => number { return x }");
        assert!(result.errors.is_empty());

        if let Some(Spanned {
            node:
                Node::FunctionExpression {
                    parameters,
                    return_type,
                    body,
                },
            ..
        }) = result.node
        {
            let params = parameters.unwrap();
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "x");
            assert_eq!(params[0].type_annotation.as_ref().unwrap(), "number");

            assert_eq!(return_type.unwrap(), "number");
            assert!(matches!(body.unwrap().node, Node::Block(_)));
        } else {
            panic!("Expected a FunctionExpression node");
        }
    }

    #[test]
    fn reports_error_on_missing_body() {
        let result = parse_expr("(x) =>");
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn reports_error_on_missing_parameters() {
        let result = parse_expr("=>");
        assert!(!result.errors.is_empty());
    }
}
