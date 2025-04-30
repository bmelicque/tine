use std::collections::HashMap;

use crate::{
    ast::{AstNode, Node},
    parser::parser::ParseError,
    types::{StructField, Type},
};

use super::TypeChecker;

impl TypeChecker {
    pub fn visit_member_expression(&mut self, node: &AstNode) -> Type {
        let Node::MemberExpression {
            ref expr,
            ref identifier,
        } = node.node
        else {
            panic!()
        };
        let Node::Identifier(ref prop) = identifier.node else {
            panic!()
        };
        let mut ty = match expr {
            Some(ref expr) => self.visit(expr),
            None => Type::Unknown,
        };
        let type_str = format!("{}", ty.clone());
        while let Type::Named { ref name, args } = ty {
            let mut substitutions = HashMap::new();
            let params = self.type_registry.get_type_params(name);
            for (i, param) in params.iter().enumerate() {
                let substitute = match args.get(i) {
                    Some(arg) => arg.clone(),
                    None => Type::Dynamic,
                };
                substitutions.insert(param, substitute);
            }
            let raw = self.type_registry.lookup(name).unwrap();
            ty = substitute_type(&raw, &substitutions);
        }
        match ty {
            Type::Struct { fields } => match fields.iter().find(|field| field.name == *prop) {
                Some(field) => field.def.clone(),
                None => {
                    self.errors.push(ParseError {
                        message: format!(
                            "Property '{}' does not exist on type '{}'",
                            prop, type_str
                        ),
                        span: node.span,
                    });
                    Type::Unknown
                }
            },
            _ => {
                self.errors.push(ParseError {
                    message: format!("Property '{}' does not exist on type '{}'", prop, type_str),
                    span: node.span,
                });
                Type::Unknown
            }
        }
    }

    // pub fn visit_tuple_indexing(&mut self, node: &AstNode) -> Type {
    //     // TODO: visit left
    //     // TODO: resolve left type to check if it is a tuple
    //     // TODO: check if number literal is in range
    //     // TODO: return type à tuple's index
    // }
}

fn substitute_type(ty: &Type, substitutions: &HashMap<&String, Type>) -> Type {
    match ty {
        // If the type is a named type, substitute it if it's in the map
        Type::Named { name, args } => {
            if let Some(substituted) = substitutions.get(name) {
                substituted.clone()
            } else {
                for arg in args {
                    // FIXME: substitute args
                }
                ty.clone()
            }
        }

        // Handle arrays recursively
        Type::Array(inner) => Type::Array(Box::new(substitute_type(inner, substitutions))),

        // Handle options recursively
        Type::Option(inner) => Type::Option(Box::new(substitute_type(inner, substitutions))),

        // Handle maps recursively
        Type::Map { key, value } => Type::Map {
            key: Box::new(substitute_type(key, substitutions)),
            value: Box::new(substitute_type(value, substitutions)),
        },

        // Handle structs recursively
        Type::Struct { fields } => Type::Struct {
            fields: fields
                .iter()
                .map(|field| StructField {
                    name: field.name.clone(),
                    def: substitute_type(&field.def, substitutions),
                    optional: field.optional,
                })
                .collect(),
        },

        // Handle other types (e.g., primitives) as is
        _ => ty.clone(),
    }
}
