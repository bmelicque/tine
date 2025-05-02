use crate::{ast, parser::parser::ParseError, types::Type};

use super::TypeChecker;

impl TypeChecker {
    pub fn visit_type(&mut self, node: &ast::Type) -> Type {
        match node {
            ast::Type::Array(array) => self.visit_array_type(array),
            ast::Type::Function(function) => self.visit_function_type(function),
            ast::Type::Map(map) => self.visit_map_type(map),
            ast::Type::Named(named) => self.visit_named_type(named),
            ast::Type::Option(option) => self.visit_option_type(option),
            ast::Type::Reference(reference) => self.visit_reference_type(reference),
            ast::Type::Result(result) => self.visit_result_type(result),
            ast::Type::Tuple(tuple) => self.visit_tuple_type(tuple),
        }
    }

    pub fn visit_array_type(&mut self, node: &ast::ArrayType) -> Type {
        let inner_type = match node.element {
            Some(ref element) => self.visit_type(element),
            None => Type::Dynamic,
        };
        Type::Array(Box::new(inner_type))
    }

    pub(super) fn visit_function_type(&mut self, node: &ast::FunctionType) -> Type {
        let params: Vec<Type> = node
            .params
            .iter()
            .map(|param| self.visit_type(param))
            .collect();

        let return_type = Box::new(self.visit_type(&node.returned));

        Type::Function {
            params,
            return_type,
        }
    }

    pub fn visit_map_type(&mut self, node: &ast::MapType) -> Type {
        let key_type = match node.key {
            Some(ref key) => self.visit_type(key),
            None => Type::Dynamic,
        };
        let value_type = match node.value {
            Some(ref value) => self.visit_type(value),
            None => Type::Dynamic,
        };

        Type::Map {
            key: Box::new(key_type),
            value: Box::new(value_type),
        }
    }

    pub fn visit_named_type(&mut self, node: &ast::NamedType) -> Type {
        let name = node.name.as_str();
        match name {
            "string" => return Type::String,
            "number" => return Type::Number,
            "boolean" => return Type::Boolean,
            "void" => return Type::Void,
            _ => {}
        }
        if self.type_registry.lookup(name).is_none() {
            self.errors.push(ParseError {
                message: format!("Type '{}' not found", name),
                span: node.span,
            });
            return Type::Unknown;
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

        let mut arg_types: Vec<Type> = args
            .iter()
            .take(arity)
            .map(|arg| self.visit_type(arg))
            .collect();
        while arg_types.len() < arity {
            arg_types.push(Type::Dynamic);
        }

        Type::Named {
            name: name.into(),
            args: arg_types,
        }
    }

    pub fn visit_option_type(&mut self, node: &ast::OptionType) -> Type {
        let inner_type = match node.base {
            Some(ref base) => self.visit_type(base),
            None => Type::Dynamic,
        };

        Type::Option(Box::new(inner_type))
    }

    pub fn visit_reference_type(&mut self, node: &ast::ReferenceType) -> Type {
        let inner_type = match node.target {
            Some(ref base) => self.visit_type(base),
            None => Type::Dynamic,
        };

        Type::Reference(Box::new(inner_type))
    }

    pub fn visit_result_type(&mut self, node: &ast::ResultType) -> Type {
        let ok_type = match node.ok {
            Some(ref ok) => self.visit_type(ok),
            None => Type::Dynamic,
        };
        let err_type = match node.error {
            Some(ref err) => self.visit_type(err),
            None => Type::Dynamic,
        };

        Type::Result {
            error: Some(Box::new(err_type)),
            ok: Box::new(ok_type),
        }
    }

    pub fn visit_tuple_type(&mut self, node: &ast::TupleType) -> Type {
        let tuple_types: Vec<Type> = node.elements.iter().map(|ty| self.visit_type(ty)).collect();
        Type::Tuple(tuple_types)
    }
}
