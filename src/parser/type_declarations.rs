use pest::iterators::Pair;

use crate::ast::{AstNode, Node, Spanned, SumTypeConstructor};

use super::{
    parser::{ParseError, ParseResult, Rule},
    types::parse_type,
    utils::{is_camel_case, is_pascal_case},
};

pub fn parse_type_declaration(pair: Pair<'static, Rule>) -> ParseResult {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let mut errors = Vec::new();

    let name = inner.next().unwrap().as_str().to_string();
    if !is_pascal_case(&name) {
        errors.push(ParseError {
            message: format!("Type name '{}' should be in PascalCase", name),
            span,
        });
    }

    let Some(def_pair) = inner.next() else {
        errors.push(ParseError {
            message: "Missing type definition".into(),
            span,
        });
        return ParseResult {
            node: Some(Spanned {
                node: Node::TypeDeclaration { name, def: None },
                span,
            }),
            errors,
        };
    };

    let def = match def_pair.as_rule() {
        Rule::struct_body => {
            let (d, mut errs) = parse_struct_body(def_pair);
            errors.append(&mut errs);
            Some(d)
        }
        Rule::sum_type => {
            let (d, mut errs) = parse_sum_type(def_pair);
            errors.append(&mut errs);
            Some(d)
        }
        // TODO: traits
        _ => unreachable!(),
    }
    .map(Box::new);

    ParseResult {
        node: Some(Spanned {
            node: Node::TypeDeclaration { name, def },
            span,
        }),
        errors,
    }
}

fn parse_struct_body(pair: Pair<'static, Rule>) -> (AstNode, Vec<ParseError>) {
    let span = pair.as_span();
    let mut errors = Vec::new();
    let mut fields = Vec::new();

    for field_pair in pair.into_inner() {
        let mut field_inner = field_pair.clone().into_inner();
        let field_name = field_inner.next().unwrap().as_str().to_string();
        let mut type_result = parse_type(field_inner.next().unwrap());
        errors.append(&mut type_result.errors);

        if !is_camel_case(&field_name) {
            errors.push(ParseError {
                message: format!("Field name '{}' should be in camelCase", field_name),
                span: field_pair.as_span(),
            });
        }

        fields.push(Spanned {
            node: (field_name, type_result.node.map(Box::new)),
            span: field_pair.as_span(),
        });
    }

    (
        Spanned {
            node: Node::Struct(fields),
            span,
        },
        errors,
    )
}

pub fn parse_sum_type(pair: Pair<'static, Rule>) -> (AstNode, Vec<ParseError>) {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let mut constructors = Vec::new();
    let mut errors = Vec::new();
    while let Some(pair) = inner.next() {
        assert!(pair.as_rule() == Rule::sum_constructor);
        let mut result = parse_sum_constructor(pair);
        errors.append(&mut result.1);
        constructors.push(result.0);
    }

    (
        Spanned {
            node: Node::Sum(constructors),
            span,
        },
        errors,
    )
}

fn parse_sum_constructor(pair: Pair<'static, Rule>) -> (SumTypeConstructor, Vec<ParseError>) {
    let mut inner = pair.into_inner();

    let mut name = None;
    let mut param = None;
    let mut errors = Vec::<ParseError>::new();
    while let Some(pair) = inner.next() {
        match pair.as_rule() {
            Rule::identifier => name = Some(pair.as_str().to_string()),
            Rule::sum_param => {
                if let Some(inner) = pair.into_inner().next() {
                    let mut result = parse_type(inner);
                    param = result.node;
                    errors.append(&mut result.errors);
                };
            }
            Rule::struct_body => {
                let mut result = parse_struct_body(pair);
                param = Some(result.0);
                errors.append(&mut result.1);
            }
            rule => unreachable!("Unexpected rule in sum_constructor (found '{:?}')", rule),
        }
    }

    (
        SumTypeConstructor {
            name: name.unwrap(),
            param,
        },
        errors,
    )
}
