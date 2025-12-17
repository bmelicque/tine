use crate::{
    ast,
    type_checker::analysis_context::type_store::TypeStore,
    types::{
        ArrayType, DuckType, FunctionType, GenericType, ListenerType, MapType, OptionType,
        ReferenceType, ResultType, SignalType, TupleType, Type, TypeId,
    },
};

use super::TypeChecker;

impl TypeChecker<'_> {
    pub fn visit_type(&mut self, node: &ast::Type) -> TypeId {
        match node {
            ast::Type::Array(array) => self.visit_array_type(array),
            ast::Type::Duck(duck) => self.visit_duck_type(duck),
            ast::Type::Function(function) => self.visit_function_type(function).into(),
            ast::Type::Listener(listener) => self.visit_listener_type(listener).into(),
            ast::Type::Map(map) => self.visit_map_type(map),
            ast::Type::Named(named) => self.visit_named_type(named),
            ast::Type::Option(option) => self.visit_option_type(option),
            ast::Type::Reference(reference) => self.visit_reference_type(reference).into(),
            ast::Type::Result(result) => self.visit_result_type(result),
            ast::Type::Signal(signal) => self.visit_signal_type(signal).into(),
            ast::Type::Tuple(tuple) => self.visit_tuple_type(tuple).into(),
        }
    }

    pub fn visit_array_type(&mut self, node: &ast::ArrayType) -> TypeId {
        let element = match node.element {
            Some(ref element) => self.visit_type(element),
            None => TypeStore::DYNAMIC,
        };
        self.intern(Type::Array(ArrayType { element }))
    }

    pub(super) fn visit_function_type(&mut self, node: &ast::FunctionType) -> TypeId {
        let params: Vec<TypeId> = node
            .params
            .iter()
            .map(|param| self.visit_type(param))
            .collect();

        let return_type = self.visit_type(&node.returned);

        self.intern(Type::Function(FunctionType {
            params,
            return_type,
        }))
    }

    fn visit_listener_type(&mut self, node: &ast::ListenerType) -> TypeId {
        let inner = self.visit_type(&node.inner);
        self.intern(Type::Listener(ListenerType { inner }))
    }

    pub fn visit_map_type(&mut self, node: &ast::MapType) -> TypeId {
        let key_type = match node.key {
            Some(ref key) => self.visit_type(key),
            None => TypeStore::DYNAMIC,
        };
        let value_type = match node.value {
            Some(ref value) => self.visit_type(value),
            None => TypeStore::DYNAMIC,
        };

        self.intern(Type::Map(MapType {
            key: key_type,
            value: value_type,
        }))
    }

    pub fn visit_named_type(&mut self, node: &ast::NamedType) -> TypeId {
        let name = node.name.as_str();
        match name {
            "string" => return TypeStore::STRING,
            "number" => return TypeStore::NUMBER,
            "boolean" => return TypeStore::BOOLEAN,
            "void" => return TypeStore::UNIT,
            _ => {}
        }
        let Some(type_ref) = self.ctx.lookup(name) else {
            self.error(format!("type '{}' not found", name), node.loc);
            return TypeStore::UNKNOWN;
        };
        let ty = type_ref.borrow().get_type();
        let def = self.resolve(ty).clone();
        match def {
            Type::Generic(t) => self.visit_generic_instance(node, &t),
            _ => ty,
        }
    }

    fn visit_generic_instance(&mut self, node: &ast::NamedType, ty: &GenericType) -> TypeId {
        let arity = ty.params.len();
        let args = node.args.clone().unwrap_or(Vec::new());

        if args.len() > arity {
            let message = format!(
                "too many arguments, expected at most {}, got {}",
                arity,
                args.len()
            );
            self.error(message, node.loc);
        }

        let mut arg_types: Vec<TypeId> = args
            .iter()
            .take(arity)
            .map(|arg| self.visit_type(arg))
            .collect();
        while arg_types.len() < arity {
            let dynamic = self.intern(Type::Dynamic);
            arg_types.push(dynamic);
        }

        self.session.types().substitute(ty.definition, &arg_types)
    }

    pub fn visit_option_type(&mut self, node: &ast::OptionType) -> TypeId {
        let some = match node.base {
            Some(ref base) => self.visit_type(base),
            None => self.intern(Type::Dynamic),
        };

        self.intern(Type::Option(OptionType { some }))
    }

    fn visit_signal_type(&mut self, node: &ast::SignalType) -> TypeId {
        let inner = self.visit_type(&node.inner);
        self.intern(Type::Signal(SignalType { inner }))
    }

    pub fn visit_reference_type(&mut self, node: &ast::ReferenceType) -> TypeId {
        let target = self.visit_type(&node.target);
        self.intern(Type::Reference(ReferenceType { target }))
    }

    pub fn visit_duck_type(&mut self, node: &ast::DuckType) -> TypeId {
        let like = self.visit_type(&node.like);
        self.intern(Type::Duck(DuckType { like }))
    }

    pub fn visit_result_type(&mut self, node: &ast::ResultType) -> TypeId {
        let ok_type = match node.ok {
            Some(ref ok) => self.visit_type(ok),
            None => TypeStore::DYNAMIC,
        };
        let err_type = match node.error {
            Some(ref err) => self.visit_type(err),
            None => TypeStore::DYNAMIC,
        };

        self.intern(Type::Result(ResultType {
            error: Some(err_type),
            ok: ok_type,
        }))
    }

    pub fn visit_tuple_type(&mut self, node: &ast::TupleType) -> TypeId {
        let elements: Vec<TypeId> = node.elements.iter().map(|ty| self.visit_type(ty)).collect();
        self.intern(Type::Tuple(TupleType { elements }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::session::Session;
    use crate::ast;
    use crate::types::StructType;
    use crate::types::Type;
    use crate::Location;
    use crate::SymbolData;
    use crate::SymbolKind;

    #[test]
    fn test_visit_array_type() {
        let session = Session::new();
        let mut checker = TypeChecker::new(&session, 0);
        let array_type = ast::ArrayType {
            element: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: "number".to_string(),
                args: None,
                loc: Location::dummy(),
            }))),
            loc: Location::dummy(),
        };

        let result = checker.visit_array_type(&array_type);
        let result = checker.resolve(result).clone();
        assert_eq!(
            result,
            Type::Array(ArrayType {
                element: TypeStore::NUMBER
            })
        );
    }

    #[test]
    fn test_visit_function_type() {
        let session = Session::new();
        let mut checker = TypeChecker::new(&session, 0);
        let function_type = ast::FunctionType {
            params: vec![
                ast::Type::Named(ast::NamedType {
                    name: "number".to_string(),
                    args: None,
                    loc: Location::dummy(),
                }),
                ast::Type::Named(ast::NamedType {
                    name: "string".to_string(),
                    args: None,
                    loc: Location::dummy(),
                }),
            ],
            returned: Box::new(ast::Type::Named(ast::NamedType {
                name: "boolean".to_string(),
                args: None,
                loc: Location::dummy(),
            })),
            loc: Location::dummy(),
        };

        let result = checker.visit_function_type(&function_type);
        let result = checker.resolve(result).clone();
        assert_eq!(
            result,
            Type::Function(FunctionType {
                params: vec![TypeStore::NUMBER, TypeStore::STRING],
                return_type: TypeStore::BOOLEAN,
            })
        );
    }

    #[test]
    fn test_visit_map_type() {
        let session = Session::new();
        let mut checker = TypeChecker::new(&session, 0);
        let map_type = ast::MapType {
            key: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: "string".to_string(),
                args: None,
                loc: Location::dummy(),
            }))),
            value: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: "number".to_string(),
                args: None,
                loc: Location::dummy(),
            }))),
            loc: Location::dummy(),
        };

        let result = checker.visit_map_type(&map_type);
        let result = checker.resolve(result).clone();
        assert_eq!(
            result,
            Type::Map(MapType {
                key: TypeStore::STRING,
                value: TypeStore::NUMBER,
            })
        );
    }

    #[test]
    fn test_visit_named_type() {
        let session = Session::new();
        let mut checker = TypeChecker::new(&session, 0);
        let def = checker.intern(Type::Struct(StructType {
            id: 7,
            fields: vec![],
        }));
        checker.ctx.register_symbol(SymbolData {
            name: "Box".into(),
            ty: def,
            kind: SymbolKind::Type { members: vec![] },
            ..Default::default()
        });

        let named_type = ast::NamedType {
            name: "Box".to_string(),
            args: None,
            loc: Location::dummy(),
        };

        let result = checker.visit_named_type(&named_type);
        let result = checker.resolve(result).clone();
        assert!(matches!(result, Type::Struct(_)));
    }

    #[test]
    fn test_visit_option_type() {
        let session = Session::new();
        let mut checker = TypeChecker::new(&session, 0);
        let option_type = ast::OptionType {
            base: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: "number".to_string(),
                args: None,
                loc: Location::dummy(),
            }))),
            loc: Location::dummy(),
        };

        let result = checker.visit_option_type(&option_type);
        let result = checker.resolve(result).clone();
        assert_eq!(
            result,
            Type::Option(OptionType {
                some: TypeStore::NUMBER
            })
        );
    }

    #[test]
    fn test_visit_reference_type() {
        let session = Session::new();
        let mut checker = TypeChecker::new(&session, 0);
        let reference_type = ast::ReferenceType {
            target: Box::new(ast::Type::Named(ast::NamedType {
                name: "string".to_string(),
                args: None,
                loc: Location::dummy(),
            })),
            loc: Location::dummy(),
        };

        let result = checker.visit_reference_type(&reference_type);
        let result = checker.resolve(result).clone();
        assert_eq!(
            result,
            ReferenceType {
                target: TypeStore::STRING,
            }
            .into()
        );
    }

    #[test]
    fn test_visit_result_type() {
        let session = Session::new();
        let mut checker = TypeChecker::new(&session, 0);
        let result_type = ast::ResultType {
            ok: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: "number".to_string(),
                args: None,
                loc: Location::dummy(),
            }))),
            error: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: "string".to_string(),
                args: None,
                loc: Location::dummy(),
            }))),
            loc: Location::dummy(),
        };

        let result = checker.visit_result_type(&result_type);
        let result = checker.resolve(result).clone();
        assert_eq!(
            result,
            Type::Result(ResultType {
                ok: TypeStore::NUMBER,
                error: Some(TypeStore::STRING),
            })
        );
    }

    #[test]
    fn test_visit_tuple_type() {
        let session = Session::new();
        let mut checker = TypeChecker::new(&session, 0);
        let tuple_type = ast::TupleType {
            elements: vec![
                ast::Type::Named(ast::NamedType {
                    name: "number".to_string(),
                    args: None,
                    loc: Location::dummy(),
                }),
                ast::Type::Named(ast::NamedType {
                    name: "string".to_string(),
                    args: None,
                    loc: Location::dummy(),
                }),
            ],
            loc: Location::dummy(),
        };

        let result = checker.visit_tuple_type(&tuple_type);
        let result = checker.resolve(result).clone();
        assert_eq!(
            result,
            Type::Tuple(TupleType {
                elements: vec![TypeStore::NUMBER, TypeStore::STRING]
            })
        );
    }
}
