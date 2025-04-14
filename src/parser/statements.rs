use pest::iterators::Pair;

use crate::ast::{AstNode, Node, Spanned};

use super::{
    expressions::parse_expression,
    parser::{ParseError, ParseResult, Rule},
};

pub fn parse_statement(pair: Pair<'static, Rule>) -> ParseResult {
    match pair.as_rule() {
        Rule::statement => {
            let inner_pair = pair.into_inner().next().unwrap();
            parse_statement(inner_pair)
        }
        Rule::variable_declaration => parse_variable_declaration(pair),
        Rule::return_statement => parse_return_statement(pair),
        Rule::expression_statement => parse_expression_statement(pair),
        _ => ParseResult::empty(),
    }
}

fn parse_variable_declaration(pair: Pair<'static, Rule>) -> ParseResult {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let mut errors = Vec::new();
    let mut name: Option<String> = None;
    let mut initializer: Option<AstNode> = None;

    while let Some(item) = inner.next() {
        match item.as_rule() {
            Rule::identifier => {
                name = Some(item.as_str().to_string());
            }
            Rule::expression => {
                let mut result = parse_expression(item);
                initializer = result.node;
                errors.append(&mut result.errors);
            }
            _ => panic!("Unexpected rule in variable declaration!"),
        }
    }
    if name.is_none() {
        errors.push(ParseError {
            message: "Value identifier expected".to_string(),
            span: span.clone(),
        });
    }
    if initializer.is_none() {
        errors.push(ParseError {
            message: "Initializer expected".to_string(),
            span,
        });
    }

    ParseResult {
        node: Some(Spanned {
            node: Node::VariableDeclaration {
                name,
                initializer: initializer.map(Box::new),
            },
            span,
        }),
        errors,
    }
}

fn parse_return_statement(pair: Pair<'static, Rule>) -> ParseResult {
    let span = pair.as_span();
    let Some(inner) = pair.clone().into_inner().next() else {
        return ParseResult {
            node: Some(Spanned {
                node: Node::ReturnStatement(None),
                span,
            }),
            errors: vec![],
        };
    };

    let result = parse_expression(inner);
    ParseResult {
        node: Some(Spanned {
            node: Node::ReturnStatement(result.node.map(Box::new)),
            span,
        }),
        errors: result.errors,
    }
}

fn parse_expression_statement(pair: Pair<'static, Rule>) -> ParseResult {
    let span = pair.as_span();
    match pair.into_inner().next() {
        Some(inner) => {
            let result = parse_expression(inner);
            ParseResult {
                node: result.node.map(|expr| Spanned {
                    node: Node::ExpressionStatement(Box::new(expr)),
                    span,
                }),
                errors: result.errors,
            }
        }
        None => ParseResult::empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{MyLanguageParser, Rule};
    use pest::Parser;

    fn parse(input: &'static str) -> ParseResult {
        let pair = MyLanguageParser::parse(Rule::statement, input)
            .unwrap()
            .next()
            .unwrap();
        parse_statement(pair)
    }

    #[test]
    fn test_valid_variable_declaration() {
        let result = parse("x := 42");
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::VariableDeclaration { name, initializer } => {
                assert_eq!(name.unwrap(), "x");
                assert!(matches!(
                    initializer.unwrap().node,
                    Node::NumberLiteral(42.0)
                ));
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

        match result.node.unwrap().node {
            Node::ReturnStatement(Some(expr)) => {
                assert!(matches!(expr.node, Node::BooleanLiteral(true)));
            }
            _ => panic!("Expected ReturnStatement with value"),
        }
    }

    #[test]
    fn test_return_statement_empty() {
        let result = parse("return");
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::ReturnStatement(None) => {}
            _ => panic!("Expected empty ReturnStatement"),
        }
    }
}
