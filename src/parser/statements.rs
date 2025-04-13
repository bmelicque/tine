use pest::iterators::Pair;

use crate::ast::Node;

use super::{
    expressions::parse_expression,
    parser::{ParseError, ParseResult, Rule},
};

pub fn parse_statement<'i>(pair: Pair<'i, Rule>) -> ParseResult<'i> {
    match pair.as_rule() {
        Rule::statement => {
            let inner_pair = pair.into_inner().next().unwrap();
            parse_statement(inner_pair)
        }
        Rule::variable_declaration => parse_variable_declaration(pair),
        Rule::return_statement => parse_return_statement(pair),
        Rule::expression_statement => parse_expression_statement(pair),
        // TODO:
        _ => ParseResult::empty(),
    }
}

fn parse_variable_declaration<'i>(pair: Pair<'i, Rule>) -> ParseResult<'i> {
    let span = pair.clone().as_span();
    let mut inner = pair.into_inner();

    let mut errors = Vec::new();
    let mut name: Option<String> = None;
    let mut initializer: Option<Box<Node>> = None;

    while let Some(item) = inner.next() {
        match item.as_rule() {
            Rule::identifier => {
                name = Some(item.as_str().to_string());
            }
            Rule::expression => {
                let mut result = parse_expression(item);
                initializer = result.node.map(Box::new);
                errors.append(&mut result.errors);
            }
            _ => panic!("Unexpected rule in variable declaration!"),
        }
    }
    if name.is_none() {
        errors.push(ParseError {
            message: "Value identifier expected".to_string(),
            span,
        });
    }
    if initializer.is_none() {
        errors.push(ParseError {
            message: "Initializer expected".to_string(),
            span,
        });
    }

    ParseResult {
        node: Some(Node::VariableDeclaration { name, initializer }),
        errors,
    }
}

fn parse_return_statement<'i>(pair: Pair<'i, Rule>) -> ParseResult<'i> {
    let Some(inner) = pair.clone().into_inner().next() else {
        return ParseResult::ok(Some(Node::ReturnStatement(None)));
    };

    let result = parse_expression(inner);
    ParseResult {
        node: Some(Node::ReturnStatement(result.node.map(Box::new))),
        errors: result.errors,
    }
}

fn parse_expression_statement<'i>(pair: Pair<'i, Rule>) -> ParseResult<'i> {
    match pair.into_inner().next() {
        Some(inner) => parse_expression(inner),
        None => ParseResult::empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{MyLanguageParser, Rule};
    use pest::Parser;

    fn parse(input: &str) -> ParseResult {
        let pair = MyLanguageParser::parse(Rule::statement, input)
            .unwrap()
            .next()
            .unwrap();
        parse_statement(pair)
    }

    #[test]
    fn test_valid_variable_declaration() {
        let result = parse("x := 42");
        assert!(
            result.errors.is_empty(),
            "Expected no errors, got {:?}",
            result.errors
        );

        match result.node {
            Some(Node::VariableDeclaration { name, initializer }) => {
                assert_eq!(name.unwrap(), "x");
                assert!(matches!(*initializer.unwrap(), Node::NumberLiteral(42.0)));
            }
            _ => panic!("Expected VariableDeclaration"),
        }
    }

    #[test]
    fn test_variable_declaration_missing_identifier() {
        let result = parse(":= 42");
        assert!(result.node.is_some());
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].message, "Value identifier expected");
    }

    #[test]
    fn test_variable_declaration_missing_initializer() {
        let result = parse("x :=");
        assert!(result.node.is_some());
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].message, "Initializer expected");
    }

    #[test]
    fn test_variable_declaration_missing_both() {
        let result = parse(":=");
        assert!(result.node.is_some());
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn test_return_statement_with_value() {
        let result = parse("return true");
        assert!(result.errors.is_empty());

        match result.node {
            Some(Node::ReturnStatement(Some(expr))) => {
                assert!(matches!(*expr, Node::BooleanLiteral(true)));
            }
            _ => panic!("Expected ReturnStatement with value"),
        }
    }

    #[test]
    fn test_return_statement_empty() {
        let result = parse("return");
        assert!(result.errors.is_empty());

        match result.node {
            Some(Node::ReturnStatement(None)) => {}
            _ => panic!("Expected empty ReturnStatement"),
        }
    }
}
