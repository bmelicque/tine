use pest::iterators::Pair;

use crate::ast::{AstNode, Node, Spanned};

use super::{
    expressions::parse_expression,
    parser::{ParseError, ParseResult, Rule},
    type_declarations::parse_type_declaration,
};

pub fn parse_statement(pair: Pair<'static, Rule>) -> ParseResult {
    match pair.as_rule() {
        Rule::statement => {
            let inner_pair = pair.into_inner().next().unwrap();
            parse_statement(inner_pair)
        }
        Rule::variable_declaration => parse_variable_declaration(pair),
        Rule::assignment => parse_assignment(pair),
        Rule::type_declaration => parse_type_declaration(pair),
        Rule::return_statement => parse_return_statement(pair),
        Rule::block => parse_block(pair),
        Rule::expression_statement => parse_expression_statement(pair),
        _ => ParseResult::empty(),
    }
}

fn parse_variable_declaration(pair: Pair<'static, Rule>) -> ParseResult {
    let (name, value, op, errors) = parse_assignment_like(pair.clone());

    ParseResult {
        node: Some(Spanned {
            node: Node::VariableDeclaration {
                name,
                op,
                initializer: value,
            },
            span: pair.as_span(),
        }),
        errors,
    }
}
fn parse_assignment(pair: Pair<'static, Rule>) -> ParseResult {
    let (name, value, _, errors) = parse_assignment_like(pair.clone());

    ParseResult {
        node: Some(Spanned {
            node: Node::Assignment { name, value },
            span: pair.as_span(),
        }),
        errors,
    }
}

// Parses variable declarations and assignments
fn parse_assignment_like(
    pair: Pair<'static, Rule>,
) -> (
    Option<String>,
    Option<Box<AstNode>>,
    String,
    Vec<ParseError>,
) {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let mut errors = Vec::new();
    let mut name: Option<String> = None;
    let mut value: Option<AstNode> = None;
    let mut op: String = "=".to_string();

    while let Some(item) = inner.next() {
        match item.as_rule() {
            Rule::value_identifier => name = Some(item.as_str().to_string()),
            Rule::expression => {
                let mut result = parse_expression(item);
                value = result.node;
                errors.append(&mut result.errors);
            }
            Rule::decl_op => op = item.as_str().to_string(),
            _ => panic!("Unexpected rule in assignment-like statement!"),
        }
    }
    if name.is_none() {
        errors.push(ParseError {
            message: "Identifier expected in lhs".to_string(),
            span: span.clone(),
        });
    }
    if value.is_none() {
        errors.push(ParseError {
            message: "Expression expected in rhs".to_string(),
            span,
        });
    }

    (name, value.map(Box::new), op, errors)
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

pub fn parse_block(pair: Pair<'static, Rule>) -> ParseResult {
    let mut errors = Vec::new();
    let mut nodes = Vec::new();

    for inner in pair.clone().into_inner() {
        let mut result = parse_statement(inner);
        if let Some(node) = result.node {
            nodes.push(node);
        }
        errors.append(&mut result.errors);
    }

    ParseResult {
        node: Some(Spanned {
            node: Node::Block(nodes),
            span: pair.as_span(),
        }),
        errors,
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
            Node::VariableDeclaration {
                name,
                op,
                initializer,
            } => {
                assert_eq!(name.unwrap(), "x");
                assert!(matches!(
                    initializer.unwrap().node,
                    Node::NumberLiteral(42.0)
                ));
                assert_eq!(op, ":=");
            }
            _ => panic!("Expected VariableDeclaration"),
        }
    }

    #[test]
    fn test_variable_declaration_missing_identifier() {
        let result = parse(":= 42");
        assert!(result.node.is_some());
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].message, "Identifier expected in lhs");
    }

    #[test]
    fn test_variable_declaration_missing_initializer() {
        let result = parse("x :=");
        assert!(result.node.is_some());
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].message, "Expression expected in rhs");
    }

    #[test]
    fn test_variable_declaration_missing_both() {
        let result = parse(":=");
        assert!(result.node.is_some());
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn test_valid_assignment() {
        let result = parse("x = 42");
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::Assignment { name, value } => {
                assert_eq!(name.unwrap(), "x");
                assert!(matches!(value.unwrap().node, Node::NumberLiteral(42.0)));
            }
            _ => panic!("Expected VariableDeclaration"),
        }
    }

    #[test]
    fn test_assignment_missing_assignee() {
        let result = parse("= 42");
        assert!(result.node.is_some());
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].message, "Identifier expected in lhs");
    }

    #[test]
    fn test_assignment_missing_value() {
        let result = parse("x =");
        assert!(result.node.is_some());
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].message, "Expression expected in rhs");
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

    #[test]
    fn parses_empty_block() {
        let result = parse("{}");
        assert!(result.errors.is_empty());

        if let Some(Spanned {
            node: Node::Block(statements),
            ..
        }) = result.node
        {
            assert!(statements.is_empty());
        } else {
            panic!("Expected a Block node");
        }
    }

    #[test]
    fn parses_single_statement_block() {
        let result = parse("{ x := 42 }");
        assert!(result.errors.is_empty());

        if let Some(Spanned {
            node: Node::Block(statements),
            ..
        }) = result.node
        {
            assert_eq!(statements.len(), 1);
            assert!(matches!(
                statements[0].node,
                Node::VariableDeclaration { .. }
            ));
        } else {
            panic!("Expected a Block node");
        }
    }

    #[test]
    fn parses_multiple_statements_block() {
        let result = parse("{ a := 1\n b := 2 }");
        assert!(result.errors.is_empty());

        if let Some(Spanned {
            node: Node::Block(statements),
            ..
        }) = result.node
        {
            assert_eq!(statements.len(), 2);
            assert!(matches!(
                statements[0].node,
                Node::VariableDeclaration { .. }
            ));
            assert!(matches!(
                statements[1].node,
                Node::VariableDeclaration { .. }
            ));
        } else {
            panic!("Expected a Block node");
        }
    }
}
