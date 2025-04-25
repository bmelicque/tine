use crate::{
    ast::{AstNode, Node},
    parser::parser::ParseError,
    types::{StructField, SumVariant, TraitMethod, Type},
};

use super::TypeChecker;

impl TypeChecker {
    pub(super) fn visit_named_type(&mut self, ast_node: &AstNode) -> Type {
        let node = &ast_node.node;
        let Node::NamedType(name) = node else {
            panic!("Expected NamedType node")
        };

        match name.as_str() {
            "string" => Type::String,
            "number" => Type::Number,
            "boolean" => Type::Boolean,
            "void" => Type::Void,
            _ => match self.type_registry.lookup(&name) {
                Some(_) => Type::Named(name.clone()),
                None => {
                    self.errors.push(ParseError {
                        message: format!("Type '{}' not found", name),
                        span: ast_node.span.clone(),
                    });
                    Type::Unknown
                }
            },
        }
    }

    pub(super) fn visit_generic_type(&mut self, ast_node: &AstNode) -> Type {
        let node = &ast_node.node;
        let Node::GenericType { name, args } = node else {
            panic!("Expected GenericType node")
        };

        let base = self.visit_named_type(name);
        match base {
            Type::Generic { .. } | Type::Unknown => {}
            _ => {
                self.errors.push(ParseError {
                    message: format!("Expected a generic type, found {:?}", base),
                    span: ast_node.span.clone(),
                });
            }
        };

        let mut generic_args = Vec::new();
        for arg in args {
            let arg_type = self.visit(arg);
            generic_args.push(arg_type);
        }

        let name = if let Node::NamedType(name) = &name.node {
            name.clone()
        } else {
            panic!("Expected NamedType node for generic type name")
        };
        Type::Generic {
            name,
            args: generic_args,
        }
    }

    pub fn visit_reference_type(&mut self, ast_node: &AstNode) -> Type {
        let node = &ast_node.node;
        let Node::ReferenceType(inner) = node else {
            panic!("Expected ReferenceType node")
        };

        let inner_type = match inner {
            Some(spanned) => self.visit(spanned),
            None => Type::Dynamic,
        };

        Type::Reference(Box::new(inner_type))
    }

    pub fn visit_option_type(&mut self, ast_node: &AstNode) -> Type {
        let node = &ast_node.node;
        let Node::OptionType(inner) = node else {
            panic!("Expected OptionType node")
        };

        let inner_type = match inner {
            Some(spanned) => self.visit(spanned),
            None => Type::Dynamic,
        };

        Type::Option(Box::new(inner_type))
    }

    pub fn visit_array_type(&mut self, ast_node: &AstNode) -> Type {
        let node = &ast_node.node;
        let Node::ArrayType(inner) = node else {
            panic!("Expected ArrayType node")
        };

        let inner_type = match inner {
            Some(spanned) => self.visit(spanned),
            None => Type::Dynamic,
        };

        Type::Array(Box::new(inner_type))
    }

    pub fn visit_map_type(&mut self, ast_node: &AstNode) -> Type {
        let node = &ast_node.node;
        let Node::MapType { key, value } = node else {
            panic!("Expected MapType node")
        };

        let key_type = match key {
            Some(spanned) => self.visit(spanned),
            None => Type::Dynamic,
        };
        let value_type = match value {
            Some(spanned) => self.visit(spanned),
            None => Type::Dynamic,
        };

        Type::Map {
            key: Box::new(key_type),
            value: Box::new(value_type),
        }
    }

    pub fn visit_result_type(&mut self, ast_node: &AstNode) -> Type {
        let node = &ast_node.node;
        let Node::ResultType { ok, err } = node else {
            panic!("Expected ResultType node")
        };

        let ok_type = match ok {
            Some(spanned) => self.visit(spanned),
            None => Type::Dynamic,
        };
        let err_type = match err {
            Some(spanned) => self.visit(spanned),
            None => Type::Dynamic,
        };

        Type::Result {
            error: Some(Box::new(err_type)),
            ok: Box::new(ok_type),
        }
    }

    pub fn visit_tuple_type(&mut self, ast_node: &AstNode) -> Type {
        let node = &ast_node.node;
        let Node::TupleType(types) = node else {
            panic!("Expected TupleType node")
        };

        let tuple_types: Vec<Type> = types
            .iter()
            .map(|ty| match ty {
                Some(spanned) => self.visit(spanned),
                None => Type::Unknown,
            })
            .collect();

        Type::Tuple(tuple_types)
    }

    pub(super) fn visit_function_type(&mut self, ast_node: &AstNode) -> Type {
        let node = &ast_node.node;
        let Node::FunctionType {
            parameters,
            return_type,
        } = node
        else {
            panic!("Expected FunctionType node")
        };

        let params: Vec<Type> = parameters
            .into_iter()
            .map(|param| self.visit(&param))
            .collect();

        let return_type = Box::new(self.visit(&return_type));

        Type::Function {
            params,
            return_type,
        }
    }

    pub(super) fn visit_struct_type(&mut self, ast_node: &AstNode) -> Type {
        let node = &ast_node.node;
        let Node::Struct(fields) = node else {
            panic!("Expected Node::Struct")
        };

        let mut struct_fields = Vec::<StructField>::new();
        for field in fields {
            let field = &field.node;
            let name = field.name.clone();
            let field_type = match field.def {
                Some(ref def) => self.visit(def),
                None => Type::Unknown,
            };
            struct_fields.push(StructField {
                name,
                def: field_type,
                optional: field.optional,
            });
        }

        Type::Struct {
            fields: struct_fields,
        }
    }

    pub(super) fn visit_sum_def(&mut self, ast_node: &AstNode) -> Type {
        let node = &ast_node.node;
        let Node::SumDef(variants) = node else {
            panic!("Expected a sum definition")
        };

        let mut variant_types = Vec::<SumVariant>::new();
        for var in variants {
            variant_types.push(SumVariant {
                name: var.name.clone(),
                def: match var.param {
                    Some(ref param) => self.visit(param),
                    None => Type::Unknown,
                },
            });
        }

        Type::Sum {
            variants: variant_types,
        }
    }

    pub(super) fn visit_trait_def(&mut self, ast_node: &AstNode) -> Type {
        let node = &ast_node.node;
        let Node::TraitDef { name, body } = node else {
            panic!("Expected a trait definition")
        };

        self.type_registry.current_self = Some(name.clone());
        let Node::Struct(methods) = &body.node else {
            panic!("Expected a struct for trait body, found {:?}", body.node)
        };

        let mut method_types = Vec::<TraitMethod>::new();
        for method in methods {
            let name = method.node.name.clone();
            let method_type = match method.node.def {
                Some(ref def) => self.visit(def),
                None => Type::Unknown,
            };
            method_types.push(TraitMethod {
                name,
                def: method_type,
            });
        }

        self.type_registry.current_self = None;

        Type::Trait {
            methods: method_types,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Node, Spanned};
    use crate::types::Type;
    use pest::Span;

    fn dummy_span() -> Span<'static> {
        let input = "dummy";
        Span::new(input, 0, input.len()).unwrap()
    }

    fn spanned(node: Node) -> Spanned<Node> {
        Spanned {
            node,
            span: dummy_span(),
        }
    }

    #[test]
    fn test_visit_named_type() {
        let mut checker = TypeChecker::new();

        // Test for built-in types
        let string_node = spanned(Node::NamedType("string".to_string()));
        let number_node = spanned(Node::NamedType("number".to_string()));
        let boolean_node = spanned(Node::NamedType("boolean".to_string()));
        let void_node = spanned(Node::NamedType("void".to_string()));

        assert!(matches!(
            checker.visit_named_type(&string_node),
            Type::String
        ));
        assert!(matches!(
            checker.visit_named_type(&number_node),
            Type::Number
        ));
        assert!(matches!(
            checker.visit_named_type(&boolean_node),
            Type::Boolean
        ));
        assert!(matches!(checker.visit_named_type(&void_node), Type::Void));

        // Test for user-defined types
        let custom_type_node = spanned(Node::NamedType("CustomType".to_string()));
        checker.type_registry.define("CustomType", Type::Number);
        let result = checker.visit_named_type(&custom_type_node);

        match result {
            Type::Named(name) => assert_eq!(name, "CustomType"),
            _ => panic!("Expected Named type, got {:?}", result),
        }
    }

    #[test]
    fn test_visit_generic_type() {
        let ast_node = spanned(Node::GenericType {
            name: Box::new(spanned(Node::NamedType("List".to_string()))),
            args: vec![
                Box::new(spanned(Node::NamedType("number".to_string()))),
                Box::new(spanned(Node::NamedType("string".to_string()))),
            ],
        });

        let mut checker = TypeChecker::new();
        checker.type_registry.define(
            "List",
            Type::Generic {
                name: "List".to_string(),
                args: vec![Type::Number, Type::String],
            },
        );
        let result = checker.visit_generic_type(&ast_node);

        match result {
            Type::Generic { name, args } => {
                assert_eq!(name, "List");
                assert_eq!(args.len(), 2);
                assert!(matches!(args[0], Type::Number));
                assert!(matches!(args[1], Type::String));
            }
            _ => panic!("Expected Generic type"),
        }
    }

    #[test]
    fn test_visit_tuple_type() {
        let ast_node = spanned(Node::TupleType(vec![
            Some(spanned(Node::NamedType("number".to_string()))),
            Some(spanned(Node::NamedType("string".to_string()))),
            None, // Represents an unknown type
        ]));

        let mut checker = TypeChecker::new();
        let result = checker.visit_tuple_type(&ast_node);

        match result {
            Type::Tuple(types) => {
                assert_eq!(types.len(), 3);
                assert!(matches!(types[0], Type::Number));
                assert!(matches!(types[1], Type::String));
                assert!(matches!(types[2], Type::Unknown));
            }
            _ => panic!("Expected Tuple type"),
        }
    }

    #[test]
    fn test_visit_function_type() {
        let ast_node = spanned(Node::FunctionType {
            parameters: vec![
                Box::new(spanned(Node::NamedType("number".to_string()))),
                Box::new(spanned(Node::NamedType("string".to_string()))),
            ],
            return_type: Box::new(spanned(Node::NamedType("boolean".to_string()))),
        });

        let mut checker = TypeChecker::new();
        let result = checker.visit_function_type(&ast_node);

        match result {
            Type::Function {
                params,
                return_type,
            } => {
                assert_eq!(params.len(), 2);
                assert!(matches!(params[0], Type::Number));
                assert!(matches!(params[1], Type::String));
                assert!(matches!(*return_type, Type::Boolean));
            }
            _ => panic!("Expected Function type"),
        }
    }

    #[test]
    fn test_visit_array_type() {
        let ast_node = spanned(Node::ArrayType(Some(Box::new(spanned(Node::NamedType(
            "number".to_string(),
        ))))));

        let mut checker = TypeChecker::new();
        let result = checker.visit_array_type(&ast_node);

        match result {
            Type::Array(inner) => {
                assert!(matches!(*inner, Type::Number));
            }
            _ => panic!("Expected Array type"),
        }
    }

    #[test]
    fn test_visit_option_type() {
        let ast_node = spanned(Node::OptionType(Some(Box::new(spanned(Node::NamedType(
            "number".to_string(),
        ))))));

        let mut checker = TypeChecker::new();
        let result = checker.visit_option_type(&ast_node);

        match result {
            Type::Option(inner) => {
                assert!(matches!(*inner, Type::Number));
            }
            _ => panic!("Expected Option type"),
        }
    }

    #[test]
    fn test_visit_map_type() {
        let ast_node = spanned(Node::MapType {
            key: Some(Box::new(spanned(Node::NamedType("string".to_string())))),
            value: Some(Box::new(spanned(Node::NamedType("number".to_string())))),
        });

        let mut checker = TypeChecker::new();
        let result = checker.visit_map_type(&ast_node);

        match result {
            Type::Map { key, value } => {
                assert!(matches!(*key, Type::String));
                assert!(matches!(*value, Type::Number));
            }
            _ => panic!("Expected Map type"),
        }
    }

    #[test]
    fn test_visit_result_type() {
        let ast_node = spanned(Node::ResultType {
            ok: Some(Box::new(spanned(Node::NamedType("number".to_string())))),
            err: Some(Box::new(spanned(Node::NamedType("string".to_string())))),
        });

        let mut checker = TypeChecker::new();
        let result = checker.visit_result_type(&ast_node);

        match result {
            Type::Result { ok, error } => {
                assert!(matches!(ok.as_ref(), Type::Number));
                assert!(matches!(*error.unwrap(), Type::String));
            }
            _ => panic!("Expected Result type"),
        }
    }
}
