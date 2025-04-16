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
        Rule::trait_type => {
            let (d, mut errs) = parse_trait(def_pair);
            errors.append(&mut errs);
            Some(d)
        }
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

fn parse_trait(pair: Pair<'static, Rule>) -> (AstNode, Vec<ParseError>) {
    let span = pair.as_span();
    let mut inner = pair.into_inner();
    let mut errors = Vec::new();

    let name_pair = inner.next().unwrap(); // Should be the identifier inside `()`
    let name = name_pair.as_str().to_string();

    if !is_pascal_case(&name) {
        errors.push(ParseError {
            message: format!("Trait name '{}' should be in PascalCase", name),
            span: name_pair.as_span(),
        });
    }

    let body_pair = inner.next().unwrap(); // Should be the struct_body after the dot
    let (body, mut body_errors) = parse_struct_body(body_pair);
    errors.append(&mut body_errors);

    (
        Spanned {
            node: Node::Trait {
                name,
                body: Box::new(body),
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
    use crate::parser::parser::MyLanguageParser;
    use crate::parser::statements::parse_statement;
    use pest::Parser;

    fn parse(input: &'static str) -> ParseResult {
        let pair = MyLanguageParser::parse(Rule::statement, input)
            .unwrap()
            .next()
            .unwrap();
        parse_statement(pair)
    }

    #[test]
    fn test_parse_struct_type_declaration() {
        let input = r#"Person :: { 
            name string
            age  number
        }"#;

        let result = parse(input);
        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::TypeDeclaration {
                name,
                def: Some(def),
            } => {
                assert_eq!(name, "Person");
                match def.node {
                    Node::Struct(fields) => {
                        assert_eq!(fields.len(), 2);
                        assert_eq!(fields[0].node.0, "name");
                        assert_eq!(fields[1].node.0, "age");
                    }
                    _ => panic!("Expected Struct node"),
                }
            }
            _ => panic!("Expected TypeDeclaration"),
        }
    }

    #[test]
    fn test_parse_sum_type_declaration() {
        let input = "Shape :: | Circle{number} | Rectangle{number}";

        let result = parse(input);

        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::TypeDeclaration {
                name,
                def: Some(def),
            } => {
                assert_eq!(name, "Shape");
                match def.node {
                    Node::Sum(constructors) => {
                        assert_eq!(constructors.len(), 2);
                        assert_eq!(constructors[0].name, "Circle");
                        assert_eq!(constructors[1].name, "Rectangle");
                    }
                    _ => panic!("Expected Sum node"),
                }
            }
            _ => panic!("Expected TypeDeclaration"),
        }
    }

    #[test]
    fn test_parse_sum_constructor_with_struct_body() {
        let input = "Result :: | Ok{value string} | Err{error string}";

        let result = parse(input);

        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::TypeDeclaration {
                name,
                def: Some(def),
            } => {
                assert_eq!(name, "Result");
                match def.node {
                    Node::Sum(constructors) => {
                        assert_eq!(constructors.len(), 2);
                        assert_eq!(constructors[0].name, "Ok");
                        assert_eq!(constructors[1].name, "Err");

                        match &constructors[0].param {
                            Some(p) => match &p.node {
                                Node::Struct(fields) => {
                                    assert_eq!(fields[0].node.0, "value");
                                }
                                _ => panic!("Expected Struct in Ok"),
                            },
                            None => panic!("Missing struct param in Ok"),
                        }
                    }
                    _ => panic!("Expected Sum node"),
                }
            }
            _ => panic!("Expected TypeDeclaration"),
        }
    }

    #[test]
    fn test_parse_trait_declaration() {
        // FIXME: test with function type
        let input = r#"Drawable :: (Self).{
                draw Self
            }"#;

        let result = parse(input);

        assert!(result.errors.is_empty());

        match result.node.unwrap().node {
            Node::TypeDeclaration {
                name,
                def: Some(def),
            } => {
                assert_eq!(name, "Drawable");
                match def.node {
                    Node::Trait {
                        name: trait_name,
                        body,
                    } => {
                        assert_eq!(trait_name, "Self");

                        match body.node {
                            Node::Struct(fields) => {
                                assert_eq!(fields.len(), 1);
                                assert_eq!(fields[0].node.0, "draw");
                            }
                            _ => panic!("Expected Struct in trait body"),
                        }
                    }
                    _ => panic!("Expected Trait node"),
                }
            }
            _ => panic!("Expected TypeDeclaration"),
        }
    }

    #[test]
    fn test_field_name_case_check() {
        let input = "BadStruct :: { NotCamel string }";

        let result = parse(input);

        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("camelCase"));
    }

    #[test]
    fn test_type_name_case_check() {
        let input = "notPascal :: { goodField string }";

        let result = parse(input);

        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("PascalCase"));
    }
}
