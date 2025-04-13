use pest::iterators::Pair;

use crate::ast::Node;

use super::{
    expressions::parse_expression,
    parser::{ParseError, ParseResult, Rule},
};

pub fn parse_statement<'i>(pair: Pair<'i, Rule>) -> ParseResult<'i, Node> {
    match pair.as_rule() {
        Rule::statement => {
            let inner_pair = pair.into_inner().next().unwrap();
            parse_statement(inner_pair)
        }
        Rule::variable_declaration => parse_variable_declaration(pair),
        Rule::return_statement => parse_return_statement(pair),
        Rule::expression_statement => parse_expression_statement(pair),
        // TODO:
        _ => Err(vec![]),
    }
}

fn parse_variable_declaration<'i>(pair: Pair<'i, Rule>) -> ParseResult<'i, Node> {
    let span = pair.clone().as_span();
    let mut inner = pair.into_inner();

    let Some(item) = inner.next() else {
        return Err(vec![
            ParseError {
                message: "Value identifier expected".to_string(),
                span,
            },
            ParseError {
                message: "Initializer expected".to_string(),
                span,
            },
        ]);
    };

    let name = match item.as_rule() {
        Rule::identifier => item.as_str().to_string(),
        Rule::expression => {
            return Err(vec![ParseError {
                message: "Value identifier expected".to_string(),
                span,
            }])
        }
        _ => panic!("Unexpected rule in variable delcaration!"),
    };

    let initializer = match item.as_rule() {
        Rule::expression => match parse_expression(item) {
            Ok(expr) => Some(expr),
            Err(e) => return Err(e),
        },
        _ => {
            return Err(vec![ParseError {
                message: "Initializer expected".to_string(),
                span,
            }])
        }
    }
    .map(Box::new);

    Ok(Node::VariableDeclaration {
        name,
        type_annotation: None,
        initializer,
    })
}

fn parse_return_statement<'i>(pair: Pair<'i, Rule>) -> ParseResult<'i, Node> {
    match pair.clone().into_inner().next() {
        Some(inner) => match parse_expression(inner) {
            Ok(expr) => Ok(Node::ReturnStatement(Some(Box::new(expr)))),
            Err(e) => Err(e),
        },
        None => Ok(Node::ReturnStatement(None)),
    }
}

fn parse_expression_statement<'i>(pair: Pair<'i, Rule>) -> ParseResult<'i, Node> {
    match pair.into_inner().next() {
        Some(inner) => Ok(Node::ExpressionStatement(Box::new(parse_expression(
            inner,
        )?))),
        None => Err(vec![]),
    }
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
        // println!("PAIR = {:#?}", pair);
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
}
