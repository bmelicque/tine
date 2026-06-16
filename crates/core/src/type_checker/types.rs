use crate::{
    ast,
    type_checker::{analysis_context::type_store::TypeStore, substitutions::Substitutions},
    types::{ArrayType, FunctionType, MapType, OptionType, ResultType, TupleType, TypeId},
    DiagnosticKind,
};

use super::TypeChecker;

impl TypeChecker<'_> {
    pub fn visit_type(&mut self, node: ast::Type) -> TypeId {
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

    pub fn visit_array_type(&mut self, node: ast::ArrayType) -> TypeId {
        let element = node
            .element
            .map_or(TypeStore::UNKNOWN, |e| self.visit_type(*e));
        self.intern(ArrayType { element })
    }

    pub(super) fn visit_function_type(&mut self, node: ast::FunctionType) -> TypeId {
        let params = node
            .params
            .into_iter()
            .map(|param| self.visit_type(param))
            .collect::<Vec<_>>();

        let return_type = node
            .returned
            .map_or(TypeStore::UNIT, |r| self.visit_type(*r));

        self.intern(FunctionType {
            params,
            return_type,
            ..Default::default()
        })
    }

    pub fn visit_map_type(&mut self, node: ast::MapType) -> TypeId {
        let key = node.key.map_or(TypeStore::DYNAMIC, |k| self.visit_type(*k));
        let value = node
            .value
            .map_or(TypeStore::DYNAMIC, |v| self.visit_type(*v));
        self.intern(MapType { key, value })
    }

    pub fn visit_named_type(&mut self, node: ast::NamedType) -> TypeId {
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
        let ty = type_symbol.as_type();
        if node.args.is_none() {
            return ty;
        }
        self.visit_type_ref(node, ty)
    }

    fn visit_type_ref(&mut self, node: ast::NamedType, ty: TypeId) -> TypeId {
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
        Substitutions::with_initial(params, &args).apply(&mut self.session.types(), ty)
    }

    pub fn visit_option_type(&mut self, node: ast::OptionType) -> TypeId {
        let some = node
            .base
            .map_or(TypeStore::DYNAMIC, |t| self.visit_type(*t));
        self.intern(OptionType { some })
    }

    pub fn visit_result_type(&mut self, node: ast::ResultType) -> TypeId {
        let ok = node.ok.map_or(TypeStore::DYNAMIC, |o| self.visit_type(*o));
        let error = node
            .error
            .map_or(TypeStore::DYNAMIC, |e| self.visit_type(*e))
            .into();
        self.intern(ResultType { ok, error })
    }

    pub fn visit_tuple_type(&mut self, node: ast::TupleType) -> TypeId {
        let elements = node
            .elements
            .into_iter()
            .map(|e| self.visit_type(e))
            .collect();
        self.intern(TupleType {
            elements,
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::session::Session;
    use crate::ast;
    use crate::type_checker::analysis_context::symbols::TypeSymbolBody;
    use crate::type_checker::test_utils::MockLoader;
    use crate::types::StructType;
    use crate::types::Type;
    use crate::Location;
    use crate::SymbolData;
    use crate::SymbolKind;

    #[test]
    fn test_visit_array_type() {
        let session = Session::new(Box::new(MockLoader));
        let mut checker = TypeChecker::new(&session, 0);
        let array_type = ast::ArrayType {
            element: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: "int".to_string(),
                args: None,
                loc: Location::dummy(),
            }))),
            loc: Location::dummy(),
        };

        let result = checker.visit_array_type(array_type);
        let result = checker.resolve(result);
        assert_eq!(
            result,
            Type::Array(ArrayType {
                element: TypeStore::INTEGER
            })
        );
    }

    #[test]
    fn test_visit_function_type() {
        let session = Session::new(Box::new(MockLoader));
        let mut checker = TypeChecker::new(&session, 0);
        let function_type = ast::FunctionType {
            params: vec![
                ast::Type::Named(ast::NamedType {
                    name: "int".to_string(),
                    args: None,
                    loc: Location::dummy(),
                }),
                ast::Type::Named(ast::NamedType {
                    name: "str".to_string(),
                    args: None,
                    loc: Location::dummy(),
                }),
            ],
            returned: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: "bool".to_string(),
                args: None,
                loc: Location::dummy(),
            }))),
            loc: Location::dummy(),
        };

        let result = checker.visit_function_type(function_type);
        let result = checker.resolve(result);
        assert_eq!(
            result,
            Type::Function(FunctionType {
                params: vec![TypeStore::INTEGER, TypeStore::STRING],
                return_type: TypeStore::BOOLEAN,
                ..Default::default()
            })
        );
    }

    #[test]
    fn test_visit_map_type() {
        let session = Session::new(Box::new(MockLoader));
        let mut checker = TypeChecker::new(&session, 0);
        let map_type = ast::MapType {
            key: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: "str".to_string(),
                args: None,
                loc: Location::dummy(),
            }))),
            value: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: "int".to_string(),
                args: None,
                loc: Location::dummy(),
            }))),
            loc: Location::dummy(),
        };

        let result = checker.visit_map_type(map_type);
        let result = checker.resolve(result);
        assert_eq!(
            result,
            Type::Map(MapType {
                key: TypeStore::STRING,
                value: TypeStore::INTEGER,
            })
        );
    }

    #[test]
    fn test_visit_named_type() {
        let session = Session::new(Box::new(MockLoader));
        let mut checker = TypeChecker::new(&session, 0);
        let def = checker.intern(Type::Struct(StructType {
            id: 7,
            ..Default::default()
        }));
        checker.ctx.register_symbol(SymbolData {
            name: "Box".into(),
            ty: def,
            kind: SymbolKind::Struct {
                body: TypeSymbolBody::Struct(vec![]),
                methods: vec![],
            },
            ..Default::default()
        });

        let named_type = ast::NamedType {
            name: "Box".to_string(),
            args: None,
            loc: Location::dummy(),
        };

        let result = checker.visit_named_type(named_type);
        let result = checker.resolve(result).clone();
        assert!(matches!(result, Type::Struct(_)));
    }

    #[test]
    fn test_visit_option_type() {
        let session = Session::new(Box::new(MockLoader));
        let mut checker = TypeChecker::new(&session, 0);
        let option_type = ast::OptionType {
            base: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: "int".to_string(),
                args: None,
                loc: Location::dummy(),
            }))),
            loc: Location::dummy(),
        };

        let result = checker.visit_option_type(option_type);
        let result = checker.resolve(result);
        assert_eq!(
            result,
            Type::Option(OptionType {
                some: TypeStore::INTEGER
            })
        );
    }

    #[test]
    fn test_visit_result_type() {
        let session = Session::new(Box::new(MockLoader));
        let mut checker = TypeChecker::new(&session, 0);
        let result_type = ast::ResultType {
            ok: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: "int".to_string(),
                args: None,
                loc: Location::dummy(),
            }))),
            error: Some(Box::new(ast::Type::Named(ast::NamedType {
                name: "str".to_string(),
                args: None,
                loc: Location::dummy(),
            }))),
            loc: Location::dummy(),
        };

        let result = checker.visit_result_type(result_type);
        let result = checker.resolve(result);
        assert_eq!(
            result,
            Type::Result(ResultType {
                ok: TypeStore::INTEGER,
                error: Some(TypeStore::STRING),
            })
        );
    }

    #[test]
    fn test_visit_tuple_type() {
        let session = Session::new(Box::new(MockLoader));
        let mut checker = TypeChecker::new(&session, 0);
        let tuple_type = ast::TupleType {
            elements: vec![
                ast::Type::Named(ast::NamedType {
                    name: "int".to_string(),
                    args: None,
                    loc: Location::dummy(),
                }),
                ast::Type::Named(ast::NamedType {
                    name: "str".to_string(),
                    args: None,
                    loc: Location::dummy(),
                }),
            ],
            loc: Location::dummy(),
        };

        let result = checker.visit_tuple_type(tuple_type);
        let result = checker.resolve(result);
        assert_eq!(
            result,
            Type::Tuple(TupleType {
                elements: vec![TypeStore::INTEGER, TypeStore::STRING],
                ..Default::default()
            })
        );
    }
}
