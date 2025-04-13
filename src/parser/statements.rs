use pest::iterators::Pair;

use crate::ast::Node;

use super::{expressions::parse_expression, parser::Rule};

pub fn parse_statement(pair: Pair<Rule>) -> Option<Node> {
    match pair.as_rule() {
        Rule::statement => {
            let inner_pair = pair.into_inner().next().unwrap();
            parse_statement(inner_pair)
        }
        Rule::variable_declaration => parse_variable_declaration(pair),
        Rule::function_declaration => parse_function_declaration(pair),
        Rule::return_statement => parse_return_statement(pair),
        Rule::expression_statement => parse_expression_statement(pair),
        _ => None,
    }
}

fn parse_variable_declaration(pair: Pair<Rule>) -> Option<Node> {
    let mut inner = pair.into_inner();
    let name = inner.next()?.as_str().to_string();

    let mut type_annotation = None;
    let mut initializer = None;

    for item in inner {
        match item.as_rule() {
            Rule::type_annotation => {
                if let Some(type_name_pair) = item.into_inner().next() {
                    type_annotation = Some(type_name_pair.as_str().to_string());
                }
            }
            Rule::expression => {
                initializer = parse_expression(item).map(Box::new);
            }
            _ => {}
        }
    }

    Some(Node::VariableDeclaration {
        name,
        type_annotation,
        initializer,
    })
}

fn parse_function_declaration(pair: Pair<Rule>) -> Option<Node> {
    let mut inner = pair.into_inner();
    let name = inner.next()?.as_str().to_string();

    let mut params = Vec::new();
    let mut return_type = None;
    let mut body = Vec::new();

    for item in inner {
        match item.as_rule() {
            Rule::parameter_list => {
                for param in item.into_inner() {
                    let mut param_inner = param.into_inner();
                    let param_name = param_inner.next()?.as_str().to_string();
                    let type_annotation = param_inner.next()?;
                    let param_type = type_annotation.into_inner().next()?.as_str().to_string();
                    params.push((param_name, param_type));
                }
            }
            Rule::type_annotation => {
                if let Some(type_name_pair) = item.into_inner().next() {
                    return_type = Some(type_name_pair.as_str().to_string());
                }
            }
            Rule::block => {
                for stmt in item.into_inner() {
                    if let Some(node) = parse_statement(stmt) {
                        body.push(node);
                    }
                }
            }
            _ => {}
        }
    }

    Some(Node::FunctionDeclaration {
        name,
        params,
        return_type,
        body,
    })
}

fn parse_return_statement(pair: Pair<Rule>) -> Option<Node> {
    let inner = pair.into_inner().next();
    let expr = inner.and_then(|p| parse_expression(p).map(Box::new));

    Some(Node::ReturnStatement(expr))
}

fn parse_expression_statement(pair: Pair<Rule>) -> Option<Node> {
    let inner = pair.into_inner().next()?;
    Some(Node::ExpressionStatement(Box::new(parse_expression(
        inner,
    )?)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{MyLanguageParser, Rule};
    use pest::Parser;

    fn parse(input: &str) -> Node {
        let pair = MyLanguageParser::parse(Rule::statement, input)
            .unwrap()
            .next()
            .unwrap();
        println!("PAIR = {:#?}", pair);
        parse_statement(pair).expect("Failed to parse statement")
    }

    #[test]
    fn test_variable_declaration() {
        let node = parse("y := true");
        match node {
            Node::VariableDeclaration {
                name,
                type_annotation,
                initializer,
            } => {
                assert_eq!(name, "y");
                assert!(type_annotation.is_none());
                assert!(matches!(*initializer.unwrap(), Node::BooleanLiteral(true)));
            }
            _ => panic!("Expected VariableDeclaration"),
        }
    }

    #[test]
    fn test_expression_statement() {
        let node = parse("foo(1, 2)");
        match node {
            Node::ExpressionStatement(expr) => match *expr {
                Node::FunctionCall { name, args } => {
                    assert_eq!(name, "foo");
                    assert_eq!(args.len(), 2);
                }
                _ => panic!("Expected FunctionCall"),
            },
            _ => panic!("Expected ExpressionStatement"),
        }
    }

    #[test]
    fn test_return_statement() {
        let node = parse("return 42");
        match node {
            Node::ReturnStatement(expr) => {
                assert!(matches!(*expr.unwrap(), Node::NumberLiteral(42.0)));
            }
            _ => panic!("Expected ReturnStatement"),
        }
    }

    #[test]
    fn test_function_declaration() {
        // FIXME:
        let node = parse("function add(a: number, b: number): number { return a + b }");
        match node {
            Node::FunctionDeclaration {
                name,
                params,
                return_type,
                body,
            } => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);
                assert_eq!(params[0], ("a".to_string(), "number".to_string()));
                assert_eq!(params[1], ("b".to_string(), "number".to_string()));
                assert_eq!(return_type.unwrap(), "number");
                assert_eq!(body.len(), 1);
                assert!(matches!(body[0], Node::ReturnStatement(_)));
            }
            _ => panic!("Expected FunctionDeclaration"),
        }
    }
}
