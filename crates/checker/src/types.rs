use tine_ast as ast;
use tine_common::diagnostics::DiagnosticKind;
use tine_types::{store::TypeStore, types};

use crate::substitutions::Substitutions;

use super::TypeChecker;

impl TypeChecker {
    pub fn visit_type(&mut self, node: ast::Type) -> types::TypeId {
        match node {
            ast::Type::Array(array) => self.visit_array_type(array),
            ast::Type::Function(function) => self.visit_function_type(function),
            ast::Type::Map(map) => self.visit_map_type(map),
            ast::Type::Named(named) => self.visit_named_type(named),
            ast::Type::Option(option) => self.visit_option_type(option),
            ast::Type::Result(result) => self.visit_result_type(result),
            ast::Type::Tuple(tuple) => self.visit_tuple_type(tuple),
        }
    }

    pub fn visit_array_type(&mut self, node: ast::ArrayType) -> types::TypeId {
        let element = node
            .element
            .map_or(TypeStore::UNKNOWN, |e| self.visit_type(*e));
        self.intern(types::TypeRef {
            inner: TypeStore::ARRAY,
            args: vec![element],
        })
    }

    pub(super) fn visit_function_type(&mut self, node: ast::FunctionType) -> types::TypeId {
        let params = node
            .params
            .into_iter()
            .map(|param| self.visit_type(param))
            .collect::<Vec<_>>();

        let return_type = node
            .returned
            .map_or(TypeStore::UNIT, |r| self.visit_type(*r));

        self.intern(types::FunctionType {
            params,
            return_type,
            ..Default::default()
        })
    }

    pub fn visit_map_type(&mut self, node: ast::MapType) -> types::TypeId {
        let key = node.key.map_or(TypeStore::DYNAMIC, |k| self.visit_type(*k));
        let value = node
            .value
            .map_or(TypeStore::DYNAMIC, |v| self.visit_type(*v));
        let symbol = self.builtin_symbol("Map").unwrap();
        self.intern(types::TypeRef {
            inner: symbol.ty(),
            args: vec![key, value],
        })
    }

    pub fn visit_named_type(&mut self, node: ast::NamedType) -> types::TypeId {
        let name = node.name.as_str();
        match name {
            "bool" => return TypeStore::BOOLEAN,
            "float" => return TypeStore::FLOAT,
            "int" => return TypeStore::INTEGER,
            "str" => return TypeStore::STRING,
            "void" => return TypeStore::UNIT,
            _ => {}
        }
        let Some(type_symbol) = self.lookup(name) else {
            let name = name.to_string();
            let error = DiagnosticKind::CannotFindName { name };
            self.error(error, node.loc);
            return TypeStore::UNKNOWN;
        };
        let ty = type_symbol.ty();
        if node.args.is_none() {
            return ty;
        }
        self.visit_type_ref(node, ty)
    }

    fn visit_type_ref(&mut self, node: ast::NamedType, ty: types::TypeId) -> types::TypeId {
        let type_ = self.resolve(ty);
        let params = type_.as_params().unwrap_or(&[]);
        let arity = params.len();
        let arg_count = node.args.as_ref().map_or(0, |a| a.len());
        if arg_count > arity {
            let error = DiagnosticKind::ArgumentCountMismatch {
                expected: arity,
                got: arg_count,
            };
            self.error(error, node.loc);
        }

        let args = node
            .args
            .unwrap_or(vec![])
            .into_iter()
            .take(arity)
            .map(|arg| self.visit_type(arg))
            .collect::<Vec<_>>();
        Substitutions::with_initial(params, &args).apply(&mut self.types, ty)
    }

    pub fn visit_option_type(&mut self, node: ast::OptionType) -> types::TypeId {
        let some = node
            .base
            .map_or(TypeStore::DYNAMIC, |t| self.visit_type(*t));
        self.intern(types::OptionType { some })
    }

    pub fn visit_result_type(&mut self, node: ast::ResultType) -> types::TypeId {
        let ok = node.ok.map_or(TypeStore::DYNAMIC, |o| self.visit_type(*o));
        let error = node
            .error
            .map_or(TypeStore::DYNAMIC, |e| self.visit_type(*e))
            .into();
        self.intern(types::ResultType { ok, error })
    }

    pub fn visit_tuple_type(&mut self, node: ast::TupleType) -> types::TypeId {
        let elements = node
            .elements
            .into_iter()
            .map(|e| self.visit_type(e))
            .collect();
        self.intern(types::TupleType {
            elements,
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tine_ast as ast;
    use tine_common::locations::Location;
    use tine_symbols::symbols::*;
    use tine_types::types::Type;

    #[test]
    fn test_visit_array_type() {
        let mut checker = TypeChecker::new();
        let array_type = ast::ArrayType {
            element: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: ast::Identifier {
                    text: "int".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            }))),
            loc: Location::dummy(),
        };

        let result = checker.visit_array_type(array_type);
        let result = checker.resolve(result);
        assert_eq!(
            result,
            Type::Ref(types::TypeRef {
                inner: TypeStore::ARRAY,
                args: vec![TypeStore::INTEGER]
            })
        );
    }

    #[test]
    fn test_visit_function_type() {
        let mut checker = TypeChecker::new();
        let function_type = ast::FunctionType {
            params: vec![
                ast::Type::Named(ast::NamedType {
                    name: ast::Identifier {
                        text: "int".to_string(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                ast::Type::Named(ast::NamedType {
                    name: ast::Identifier {
                        text: "str".to_string(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            ],
            returned: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: ast::Identifier {
                    text: "bool".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            }))),
            loc: Location::dummy(),
        };

        let result = checker.visit_function_type(function_type);
        let result = checker.resolve(result);
        assert_eq!(
            result,
            Type::Function(types::FunctionType {
                params: vec![TypeStore::INTEGER, TypeStore::STRING],
                return_type: TypeStore::BOOLEAN,
                ..Default::default()
            })
        );
    }

    #[test]
    fn test_visit_map_type() {
        let mut checker = TypeChecker::new();
        let map_type = ast::MapType {
            key: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: ast::Identifier {
                    text: "str".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            }))),
            value: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: ast::Identifier {
                    text: "int".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            }))),
            loc: Location::dummy(),
        };

        let result = checker.visit_map_type(map_type);
        let result = checker.resolve(result);
        let symbol = checker.builtin_symbol("Map").unwrap();
        assert_eq!(
            result,
            Type::Ref(types::TypeRef {
                inner: symbol.ty(),
                args: vec![TypeStore::STRING, TypeStore::INTEGER,]
            })
        );
    }

    #[test]
    fn test_visit_named_type() {
        let mut checker = TypeChecker::new();
        let def = checker.intern(types::StructType {
            id: 7,
            ..Default::default()
        });
        let id = checker.symbols.insert::<StructSymbolId>(StructSymbol {
            name: "Box".into(),
            ty: def,
            body: TypeSymbolBody::Struct(vec![]),
            ..Default::default()
        });
        checker.current_scope().bind("Box".into(), id.into());

        let named_type = ast::NamedType {
            name: ast::Identifier {
                text: "Box".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = checker.visit_named_type(named_type);
        let result = checker.resolve(result).clone();
        assert!(
            matches!(result, Type::Struct(_)),
            "expected struct, got {:?}",
            result
        );
    }

    #[test]
    fn test_visit_option_type() {
        let mut checker = TypeChecker::new();
        let option_type = ast::OptionType {
            base: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: ast::Identifier {
                    text: "int".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            }))),
            loc: Location::dummy(),
        };

        let result = checker.visit_option_type(option_type);
        let result = checker.resolve(result);
        assert_eq!(
            result,
            Type::Option(types::OptionType {
                some: TypeStore::INTEGER
            })
        );
    }

    #[test]
    fn test_visit_result_type() {
        let mut checker = TypeChecker::new();
        let result_type = ast::ResultType {
            ok: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: ast::Identifier {
                    text: "int".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            }))),
            error: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: ast::Identifier {
                    text: "str".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            }))),
            loc: Location::dummy(),
        };

        let result = checker.visit_result_type(result_type);
        let result = checker.resolve(result);
        assert_eq!(
            result,
            Type::Result(types::ResultType {
                ok: TypeStore::INTEGER,
                error: Some(TypeStore::STRING),
            })
        );
    }

    #[test]
    fn test_visit_tuple_type() {
        let mut checker = TypeChecker::new();
        let tuple_type = ast::TupleType {
            elements: vec![
                ast::Type::Named(ast::NamedType {
                    name: ast::Identifier {
                        text: "int".to_string(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                ast::Type::Named(ast::NamedType {
                    name: ast::Identifier {
                        text: "str".to_string(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            ],
            loc: Location::dummy(),
        };

        let result = checker.visit_tuple_type(tuple_type);
        let result = checker.resolve(result);
        assert_eq!(
            result,
            Type::Tuple(types::TupleType {
                elements: vec![TypeStore::INTEGER, TypeStore::STRING],
                ..Default::default()
            })
        );
    }
}
