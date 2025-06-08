use crate::{ast, parser::parser::ParseError, types};

use super::TypeChecker;

impl TypeChecker {
    pub fn visit_type(&mut self, node: &ast::Type) -> types::Type {
        match node {
            ast::types::Type::Array(array) => self.visit_array_type(array),
            ast::Type::Function(function) => self.visit_function_type(function).into(),
            ast::Type::Map(map) => self.visit_map_type(map),
            ast::Type::Named(named) => self.visit_named_type(named),
            ast::Type::Option(option) => self.visit_option_type(option),
            ast::Type::Reference(reference) => self.visit_reference_type(reference),
            ast::Type::Result(result) => self.visit_result_type(result),
            ast::Type::Tuple(tuple) => self.visit_tuple_type(tuple).into(),
        }
    }

    pub fn visit_array_type(&mut self, node: &ast::ArrayType) -> types::Type {
        let inner_type = match node.element {
            Some(ref element) => self.visit_type(element),
            None => types::Type::Dynamic,
        };
        types::Type::Array(types::ArrayType {
            element: Box::new(inner_type),
        })
    }

    pub(super) fn visit_function_type(&mut self, node: &ast::FunctionType) -> types::FunctionType {
        let params: Vec<types::Type> = node
            .params
            .iter()
            .map(|param| self.visit_type(param))
            .collect();

        let return_type = Box::new(self.visit_type(&node.returned));

        types::FunctionType {
            params,
            return_type,
        }
    }

    pub fn visit_map_type(&mut self, node: &ast::MapType) -> types::Type {
        let key_type = match node.key {
            Some(ref key) => self.visit_type(key),
            None => types::Type::Dynamic,
        };
        let value_type = match node.value {
            Some(ref value) => self.visit_type(value),
            None => types::Type::Dynamic,
        };

        types::Type::Map(types::MapType {
            key: Box::new(key_type),
            value: Box::new(value_type),
        })
    }

    pub fn visit_named_type(&mut self, node: &ast::NamedType) -> types::Type {
        let name = node.name.as_str();
        match name {
            "string" => return types::Type::String,
            "number" => return types::Type::Number,
            "boolean" => return types::Type::Boolean,
            "void" => return types::Type::Void,
            _ => {}
        }
        if self.type_registry.lookup(name).is_none() {
            self.errors.push(ParseError {
                message: format!("Type '{}' not found", name),
                span: node.span,
            });
            return types::Type::Unknown;
        }

        let arity = self.type_registry.get_type_params(name).len();
        let args = node.args.clone().unwrap_or(Vec::new());

        if args.len() > arity {
            self.errors.push(ParseError {
                message: format!(
                    "Too many arguments, expected at most {}, got {}",
                    arity,
                    args.len()
                ),
                span: node.span,
            });
        }

        let mut arg_types: Vec<types::Type> = args
            .iter()
            .take(arity)
            .map(|arg| self.visit_type(arg))
            .collect();
        while arg_types.len() < arity {
            arg_types.push(types::Type::Dynamic);
        }

        types::Type::Named(types::NamedType {
            name: name.into(),
            args: arg_types,
        })
    }

    pub fn visit_option_type(&mut self, node: &ast::OptionType) -> types::Type {
        let inner_type = match node.base {
            Some(ref base) => self.visit_type(base),
            None => types::Type::Dynamic,
        };

        types::Type::Option(types::OptionType {
            some: Box::new(inner_type),
        })
    }

    pub fn visit_reference_type(&mut self, node: &ast::ReferenceType) -> types::Type {
        let inner_type = match node.target {
            Some(ref base) => self.visit_type(base),
            None => types::Type::Dynamic,
        };

        types::Type::Reference(types::ReferenceType {
            target: Box::new(inner_type),
        })
    }

    pub fn visit_result_type(&mut self, node: &ast::ResultType) -> types::Type {
        let ok_type = match node.ok {
            Some(ref ok) => self.visit_type(ok),
            None => types::Type::Dynamic,
        };
        let err_type = match node.error {
            Some(ref err) => self.visit_type(err),
            None => types::Type::Dynamic,
        };

        types::Type::Result(types::ResultType {
            error: Some(Box::new(err_type)),
            ok: Box::new(ok_type),
        })
    }

    pub fn visit_tuple_type(&mut self, node: &ast::TupleType) -> types::TupleType {
        let elements: Vec<types::Type> =
            node.elements.iter().map(|ty| self.visit_type(ty)).collect();
        types::TupleType { elements }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;
    use crate::types::Type;

    fn create_type_checker() -> TypeChecker {
        TypeChecker {
            errors: Vec::new(),
            symbols: Default::default(),
            type_registry: Default::default(),
        }
    }

    fn dummy_span() -> pest::Span<'static> {
        pest::Span::new("_", 0, 0).unwrap()
    }

    #[test]
    fn test_visit_array_type() {
        let mut checker = create_type_checker();
        let array_type = ast::ArrayType {
            element: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: "number".to_string(),
                args: None,
                span: dummy_span(),
            }))),
            span: dummy_span(),
        };

        let result = checker.visit_array_type(&array_type);
        assert_eq!(
            result,
            types::Type::Array(types::ArrayType {
                element: Box::new(Type::Number)
            })
        );
    }

    #[test]
    fn test_visit_function_type() {
        let mut checker = create_type_checker();
        let function_type = ast::FunctionType {
            params: vec![
                ast::Type::Named(ast::NamedType {
                    name: "number".to_string(),
                    args: None,
                    span: dummy_span(),
                }),
                ast::Type::Named(ast::NamedType {
                    name: "string".to_string(),
                    args: None,
                    span: dummy_span(),
                }),
            ],
            returned: Box::new(ast::Type::Named(ast::NamedType {
                name: "boolean".to_string(),
                args: None,
                span: dummy_span(),
            })),
            span: dummy_span(),
        };

        let result = checker.visit_function_type(&function_type);
        assert_eq!(
            result,
            types::FunctionType {
                params: vec![Type::Number, Type::String],
                return_type: Box::new(Type::Boolean),
            }
        );
    }

    #[test]
    fn test_visit_map_type() {
        let mut checker = create_type_checker();
        let map_type = ast::MapType {
            key: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: "string".to_string(),
                args: None,
                span: dummy_span(),
            }))),
            value: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: "number".to_string(),
                args: None,
                span: dummy_span(),
            }))),
            span: dummy_span(),
        };

        let result = checker.visit_map_type(&map_type);
        assert_eq!(
            result,
            types::Type::Map(types::MapType {
                key: Box::new(Type::String),
                value: Box::new(Type::Number),
            })
        );
    }

    #[test]
    fn test_visit_named_type() {
        let mut checker = create_type_checker();
        checker.type_registry.define(
            "Box",
            types::Type::Struct(types::StructType { fields: vec![] }),
            None,
        );

        let named_type = ast::NamedType {
            name: "Box".to_string(),
            args: None,
            span: dummy_span(),
        };

        let result = checker.visit_named_type(&named_type);
        assert_eq!(
            result,
            types::Type::Named(types::NamedType {
                name: "Box".to_string(),
                args: vec![],
            })
        );
    }

    #[test]
    fn test_visit_option_type() {
        let mut checker = create_type_checker();
        let option_type = ast::OptionType {
            base: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: "number".to_string(),
                args: None,
                span: dummy_span(),
            }))),
            span: dummy_span(),
        };

        let result = checker.visit_option_type(&option_type);
        assert_eq!(
            result,
            types::Type::Option(types::OptionType {
                some: Box::new(Type::Number)
            })
        );
    }

    #[test]
    fn test_visit_reference_type() {
        let mut checker = create_type_checker();
        let reference_type = ast::ReferenceType {
            target: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: "string".to_string(),
                args: None,
                span: dummy_span(),
            }))),
            span: dummy_span(),
        };

        let result = checker.visit_reference_type(&reference_type);
        assert_eq!(
            result,
            types::Type::Reference(types::ReferenceType {
                target: Box::new(Type::String)
            })
        );
    }

    #[test]
    fn test_visit_result_type() {
        let mut checker = create_type_checker();
        let result_type = ast::ResultType {
            ok: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: "number".to_string(),
                args: None,
                span: dummy_span(),
            }))),
            error: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: "string".to_string(),
                args: None,
                span: dummy_span(),
            }))),
            span: dummy_span(),
        };

        let result = checker.visit_result_type(&result_type);
        assert_eq!(
            result,
            types::Type::Result(types::ResultType {
                ok: Box::new(Type::Number),
                error: Some(Box::new(Type::String)),
            })
        );
    }

    #[test]
    fn test_visit_tuple_type() {
        let mut checker = create_type_checker();
        let tuple_type = ast::TupleType {
            elements: vec![
                ast::Type::Named(ast::NamedType {
                    name: "number".to_string(),
                    args: None,
                    span: dummy_span(),
                }),
                ast::Type::Named(ast::NamedType {
                    name: "string".to_string(),
                    args: None,
                    span: dummy_span(),
                }),
            ],
            span: dummy_span(),
        };

        let result = checker.visit_tuple_type(&tuple_type);
        assert_eq!(
            result,
            types::TupleType {
                elements: vec![Type::Number, Type::String]
            }
        );
    }
}
