use pest::iterators::Pair;

use super::{
    expressions::parse_expression,
    parser::{ParseError, ParseResult, Rule},
    types::parse_type,
};
use crate::ast::{AstNode, FieldAssignment, MapEntry, Node, Spanned};

pub fn parse_composite_literal(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::composite_literal);
    let pair = pair.into_inner().next().unwrap();
    match pair.as_rule() {
        Rule::map_literal => parse_map_literal(pair),
        Rule::array_literal => parse_array_literal(pair),
        Rule::option_literal => parse_option_literal(pair),
        Rule::struct_literal => parse_struct_literal(pair),

        _ => panic!("Not implemented, got rule: {:?}", pair.as_rule()),
    }
}

fn parse_map_literal(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::map_literal);
    let span = pair.as_span();
    let mut inner = pair.into_inner();
    let mut errors = Vec::new();

    let type_result = parse_type(inner.next().unwrap());
    errors.extend(type_result.errors);

    let map_body = inner.next().unwrap();
    assert!(map_body.as_rule() == Rule::map_body);
    let mut entries = Vec::new();
    for entry_pair in map_body.into_inner() {
        let (entry, errs) = parse_map_entry(entry_pair);
        entries.push(entry);
        errors.extend(errs);
    }

    ParseResult {
        node: Some(Spanned {
            node: Node::MapLiteral {
                ty: Box::new(type_result.node.unwrap()),
                entries,
            },
            span,
        }),
        errors,
    }
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

fn parse_array_literal(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::array_literal);
    let span = pair.as_span();
    let mut inner = pair.into_inner();
    let mut errors = Vec::new();

    let type_result = parse_type(inner.next().unwrap());
    errors.extend(type_result.errors);

    let result = parse_array_literal_body(inner.next().unwrap());
    let elements = result.0;
    errors.extend(result.1);

    ParseResult {
        node: Some(Spanned {
            node: Node::ArrayLiteral {
                ty: Box::new(type_result.node.unwrap()),
                elements,
            },
            span,
        }),
        errors,
    }
}

pub fn parse_anonymous_array_literal(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::array_literal_body);
    let span = pair.as_span();
    let result = parse_array_literal_body(pair.into_inner().next().unwrap());
    ParseResult {
        node: Some(Spanned {
            node: Node::AnonymousArrayLiteral(result.0),
            span,
        }),
        errors: result.1,
    }
}

fn parse_array_literal_body(pair: Pair<'static, Rule>) -> (Vec<AstNode>, Vec<ParseError>) {
    assert!(pair.as_rule() == Rule::array_literal_body);
    let mut errors = Vec::new();
    let mut elements = Vec::new();
    for element_pair in pair.into_inner() {
        let result = parse_expression(element_pair);
        if let Some(expr) = result.node {
            elements.push(expr);
        }
        errors.extend(result.errors);
    }
    (elements, errors)
}

fn parse_option_literal(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::option_literal);
    let span = pair.as_span();
    let mut inner = pair.into_inner();
    let mut errors = Vec::new();

    let type_result = parse_type(inner.next().unwrap());
    errors.extend(type_result.errors);

    let value = inner
        .next()
        .and_then(|pair| {
            let result = parse_expression(pair);
            errors.extend(result.errors);
            result.node
        })
        .map(Box::new);

    ParseResult {
        node: Some(Spanned {
            node: Node::OptionLiteral {
                ty: Box::new(type_result.node.unwrap()),
                value,
            },
            span,
        }),
        errors,
    }
}

fn parse_struct_literal(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::struct_literal);
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let struct_type = parse_type(inner.next().unwrap());
    let body_result = parse_struct_instance_body(inner.next().unwrap());
    let fields = body_result.0;

    let mut errors = struct_type.errors;
    errors.extend(body_result.1);

    ParseResult {
        node: Some(Spanned {
            node: Node::StructLiteral {
                struct_type: Box::new(struct_type.node.unwrap()),
                fields,
            },
            span,
        }),
        errors,
    }
}

pub fn parse_anonymous_struct_literal(pair: Pair<'static, Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::struct_literal_body);
    let span = pair.as_span();
    let (fields, errors) = parse_struct_instance_body(pair.into_inner().next().unwrap());

    ParseResult {
        node: Some(Spanned {
            node: Node::AnonymousStructLiteral(fields),
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

    fn parse(input: &'static str) -> ParseResult {
        let pair = MyLanguageParser::parse(Rule::composite_literal, input)
            .unwrap()
            .next()
            .unwrap();
        parse_composite_literal(pair)
    }

    #[test]
    fn test_parse_map_literal() {
        let input = r#"string#number{ "key": 42, "another_key": 99 }"#;

        let result = parse(input);
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::MapLiteral { ty: _, entries } => {
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
            _ => panic!("Expected MapLiteral"),
        }
    }

    #[test]
    fn test_parse_array_literal() {
        let input = r#"[]number{1, 2, 3}"#;

        let result = parse(input);
        assert!(result.errors.is_empty());

        let Node::ArrayLiteral { ty, elements } = result.node.unwrap().node else {
            panic!("Expected ArrayLiteral");
        };

        let Node::ArrayType(Some(boxed)) = ty.node else {
            panic!("Expected ArrayType");
        };
        assert!(matches!(boxed.node, Node::NamedType(ref name) if name == "number"));

        assert_eq!(elements.len(), 3);

        assert!(matches!(elements[0].node, Node::NumberLiteral(1.0)));
        assert!(matches!(elements[1].node, Node::NumberLiteral(2.0)));
        assert!(matches!(elements[2].node, Node::NumberLiteral(3.0)));
    }

    #[test]
    fn test_parse_option_literal() {
        let input = r#"?number{42}"#;

        let result = parse(input);
        assert!(result.errors.is_empty());

        let Node::OptionLiteral { ty, value } = result.node.unwrap().node else {
            panic!("Expected OptionLiteral");
        };
        let Node::OptionType(Some(boxed)) = ty.node else {
            panic!("Expected OptionType");
        };
        assert!(
            matches!(boxed.node, Node::NamedType(ref name) if name == "number"),
            "Expected NamedType, got {:?}",
            boxed.node
        );
        assert!(matches!(value.unwrap().node, Node::NumberLiteral(42.0)));
    }

    #[test]
    fn test_parse_struct_literal() {
        let input = r#"User{ name: "John", age: 21 }"#;

        let result = parse(input);
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::StructLiteral {
                struct_type,
                fields,
            } => {
                assert!(matches!(struct_type.node, Node::NamedType(ref name) if name == "User"));
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
            _ => panic!("Expected StructLiteral"),
        }
    }
}
