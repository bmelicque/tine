use pest::iterators::Pair;

use super::{
    expressions::parse_expression,
    parser::{ParseError, ParseResult, Rule},
    types::parse_type,
};
use crate::ast::{AstNode, FieldAssignment, MapEntry, Node, Spanned};

pub fn parse_type_instantiation(pair: Pair<'static, Rule>) -> ParseResult {
    match pair.as_rule() {
        Rule::map_instantiation => parse_map_instantiation(pair),
        Rule::unary_instantiation => parse_unary_instantiation(pair),
        Rule::struct_instantiation => parse_struct_instantiation(pair),
        _ => unreachable!(),
    }
}

fn parse_map_instantiation(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::map_instantiation);
    let mut errors = Vec::new();
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let type_result = parse_type(inner.next().unwrap());
    errors.extend(type_result.errors);
    let type_is_map = match type_result.node {
        Some(Spanned {
            node:
                Node::BinaryType {
                    left: _,
                    ref operator,
                    right: _,
                },
            ..
        }) => operator == "#",
        _ => false,
    };
    if !type_is_map {
        errors.push(ParseError {
            message: "Expected a map type".to_string(),
            span: match type_result.node {
                Some(ref spanned) => spanned.span,
                None => span,
            },
        });
    }

    let map_body = inner.next().unwrap();
    let body_result = parse_map_body(map_body);
    errors.extend(body_result.1);

    ParseResult {
        node: Some(Spanned {
            node: Node::MapInstantiation {
                ty: type_result.node.map(Box::new),
                entries: body_result.0,
            },
            span,
        }),
        errors,
    }
}

fn parse_map_body(pair: Pair<'static, Rule>) -> (Vec<Spanned<MapEntry>>, Vec<ParseError>) {
    let mut entries = Vec::new();
    let mut errors = Vec::new();

    for entry_pair in pair.into_inner() {
        let result = parse_map_entry(entry_pair);
        entries.push(result.0);
        errors.extend(result.1);
    }

    (entries, errors)
}

fn parse_map_entry(pair: Pair<'static, Rule>) -> (Spanned<MapEntry>, Vec<ParseError>) {
    assert!(pair.as_rule() == Rule::map_entry);
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let key_pair = inner.next().unwrap().into_inner().next().unwrap();
    let key = parse_expression(key_pair);
    let value = parse_expression(inner.next().unwrap());

    let mut errors = key.errors;
    errors.extend(value.errors);

    (
        Spanned {
            node: MapEntry {
                key: Box::new(key.node.unwrap()),
                value: Box::new(value.node.unwrap()),
            },
            span,
        },
        errors,
    )
}

fn parse_unary_instantiation(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::unary_instantiation);
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let unary_type = parse_type(inner.next().unwrap());
    let body_result = parse_unary_body(inner.next().unwrap());
    let body = body_result.0;

    let mut errors = unary_type.errors;
    errors.extend(body_result.1);

    ParseResult {
        node: Some(Spanned {
            node: Node::UnaryInstantiation {
                unary_type: Box::new(unary_type.node.unwrap()),
                body,
            },
            span,
        }),
        errors,
    }
}

fn parse_unary_body(pair: Pair<'static, Rule>) -> (Vec<AstNode>, Vec<ParseError>) {
    let mut elements = Vec::new();
    let mut errors = Vec::new();

    for expr_pair in pair.into_inner() {
        let result = parse_expression(expr_pair);
        if let Some(expr) = result.node {
            elements.push(expr);
        }
        errors.extend(result.errors);
    }

    (elements, errors)
}

fn parse_struct_instantiation(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::struct_instantiation);
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let struct_type = parse_type(inner.next().unwrap());
    let body_result = parse_struct_instance_body(inner.next().unwrap());
    let fields = body_result.0;

    let mut errors = struct_type.errors;
    errors.extend(body_result.1);

    ParseResult {
        node: Some(Spanned {
            node: Node::StructInstantiation {
                struct_type: Box::new(struct_type.node.unwrap()),
                fields,
            },
            span,
        }),
        errors,
    }
}

fn parse_struct_instance_body(
    pair: Pair<'static, Rule>,
) -> (Vec<Spanned<FieldAssignment>>, Vec<ParseError>) {
    let mut fields = Vec::new();
    let mut errors = Vec::new();

    for field_pair in pair.into_inner() {
        let result = parse_field_assignment(field_pair);
        fields.push(result.0);
        errors.extend(result.1);
    }

    (fields, errors)
}

fn parse_field_assignment(
    pair: Pair<'static, Rule>,
) -> (Spanned<FieldAssignment>, Vec<ParseError>) {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let name = inner.next().unwrap().as_str().to_string();
    let value = parse_expression(inner.next().unwrap());

    let errors = value.errors;

    (
        Spanned {
            node: FieldAssignment {
                name,
                value: Box::new(value.node.unwrap()),
            },
            span,
        },
        errors,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Node;
    use crate::parser::parser::{MyLanguageParser, Rule};
    use pest::Parser;

    fn parse(input: &'static str, rule: Rule) -> ParseResult {
        let pair = MyLanguageParser::parse(rule, input)
            .unwrap()
            .next()
            .unwrap();
        parse_type_instantiation(pair)
    }

    #[test]
    fn test_parse_map_instantiation() {
        let input = r#"string#number{ "key": 42, "another_key": 99 }"#;

        let result = parse(input, Rule::map_instantiation);
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::MapInstantiation { ty, entries } => {
                assert!(
                    matches!(ty.unwrap().node, Node::BinaryType { left:_, operator, right:_ } if  operator == "#" )
                );
                assert_eq!(entries.len(), 2);

                assert!(
                    matches!(entries[0].node.key.node, Node::StringLiteral(ref s) if s == "key")
                );
                assert!(matches!(
                    entries[0].node.value.node,
                    Node::NumberLiteral(42.0)
                ));
                assert!(
                    matches!(entries[1].node.key.node, Node::StringLiteral(ref s) if s == "another_key")
                );
                assert!(matches!(
                    entries[1].node.value.node,
                    Node::NumberLiteral(99.0)
                ));
            }
            _ => panic!("Expected MapInstantiation"),
        }
    }

    #[test]
    fn test_parse_unary_instantiation() {
        let input = r#"[]number{1, 2, 3}"#;

        let result = parse(input, Rule::unary_instantiation);
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::UnaryInstantiation { unary_type, body } => {
                match unary_type.node {
                    Node::UnaryType {
                        ref operator,
                        ref inner,
                    } => {
                        assert_eq!(operator, "[]");
                        assert!(
                            matches!(inner.clone().unwrap().node, Node::Identifier(ref name) if name == "number")
                        );
                    }
                    _ => panic!("Expected UnaryType"),
                }
                assert_eq!(body.len(), 3);

                assert!(matches!(body[0].node, Node::NumberLiteral(1.0)));
                assert!(matches!(body[1].node, Node::NumberLiteral(2.0)));
                assert!(matches!(body[2].node, Node::NumberLiteral(3.0)));
            }
            _ => panic!("Expected UnaryInstantiation"),
        }
    }

    #[test]
    fn test_parse_struct_instantiation() {
        let input = r#"User{ name: "John", age: 21 }"#;

        let result = parse(input, Rule::struct_instantiation);
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::StructInstantiation {
                struct_type,
                fields,
            } => {
                assert!(matches!(struct_type.node, Node::Identifier(ref name) if name == "User"));
                assert_eq!(fields.len(), 2);

                assert_eq!(fields[0].node.name, "name");
                assert!(matches!(
                    fields[0].node.value.node,
                    Node::StringLiteral(ref s) if s == "John"
                ));
                assert_eq!(fields[1].node.name, "age");
                assert!(matches!(
                    fields[1].node.value.node,
                    Node::NumberLiteral(21.0)
                ));
            }
            _ => panic!("Expected StructInstantiation"),
        }
    }
}
