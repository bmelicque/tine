use core::panic;
use std::collections::{HashMap, HashSet};

use crate::{
    ast,
    diagnostics::DiagnosticKind,
    type_checker::analysis_context::type_store::TypeStore,
    types::{self, StructField, TypeId},
    Location,
};

use super::super::TypeChecker;

enum ExpectedFields {
    Tuple(Vec<TypeId>),
    Struct(Vec<StructField>),
    MapEntries(types::MapType),
    Any,
}

struct ConstructorVisit {
    pub constructor_type: TypeId,
    pub type_params: Vec<TypeId>,
    pub substitutions: HashMap<types::TypeParam, TypeId>,
    pub expected_fields: Option<ExpectedFields>,
}

impl Default for ConstructorVisit {
    fn default() -> Self {
        Self {
            constructor_type: TypeStore::UNKNOWN,
            type_params: vec![],
            substitutions: HashMap::new(),
            expected_fields: None,
        }
    }
}

impl TypeChecker<'_> {
    pub fn visit_constructor_literal(&mut self, node: &ast::ConstructorLiteral) -> TypeId {
        let ConstructorVisit {
            constructor_type,
            type_params,
            mut substitutions,
            expected_fields,
        } = self.visit_constructor(&node.constructor);

        self.check_literal_body(&node.body, expected_fields, &mut substitutions);

        let ty =
            self.resolve_constructor_type(constructor_type, type_params, node.loc, &substitutions);
        self.ctx.save_expression_type(node.loc, ty)
    }

    fn visit_constructor(&mut self, node: &ast::Constructor) -> ConstructorVisit {
        match node {
            ast::Constructor::Named(named) => {
                let (ty, type_params, substitutions) = self.resolve_constructor_name(named);
                let expected_fields = self.get_expected_fields(ty, named.loc);

                ConstructorVisit {
                    constructor_type: ty,
                    type_params,
                    substitutions,
                    expected_fields,
                }
            }
            ast::Constructor::Map(map) => {
                let map_type_id = self.visit_map_type(map);
                let types::Type::Map(map_type) = self.resolve(map_type_id) else {
                    panic!()
                };
                ConstructorVisit {
                    constructor_type: map_type_id,
                    expected_fields: Some(ExpectedFields::MapEntries(map_type)),
                    ..Default::default()
                }
            }
            ast::Constructor::Variant(variant) => {
                let (ty, type_params, substitutions) =
                    self.resolve_constructor_name(&variant.enum_name);

                let expected_fields = self.get_variant_expected_fields(variant, ty, node.loc());

                ConstructorVisit {
                    constructor_type: ty,
                    type_params,
                    substitutions,
                    expected_fields,
                }
            }
            ast::Constructor::Invalid(_) => ConstructorVisit::default(),
        }
    }

    /// Tries to resolve the name of the constructor being called.
    /// Returns the type definition and a substitution map.
    /// `GenericType` definitions will be unwrapped to the inner definition type.
    fn resolve_constructor_name(
        &mut self,
        name: &ast::NamedType,
    ) -> (TypeId, Vec<TypeId>, HashMap<types::TypeParam, TypeId>) {
        let name_str = name.name.as_str();
        let ty = match name_str {
            "bool" => TypeStore::BOOLEAN,
            "float" => TypeStore::FLOAT,
            "int" => TypeStore::INTEGER,
            "str" => TypeStore::STRING,
            "void" => TypeStore::UNIT,
            _ => match self.lookup(name_str) {
                Some(symbol) => symbol.borrow().get_type(),
                None => {
                    let error = DiagnosticKind::CannotFindName {
                        name: name_str.to_string(),
                    };
                    self.error(error, name.loc);
                    TypeStore::UNKNOWN
                }
            },
        };

        match self.resolve(ty) {
            types::Type::Generic(g) => {
                self.check_type_args_count(name, &g);
                let substitutions =
                    self.get_explicit_substitutions(&name.args, &g.params, name.loc);
                (g.definition, g.params, substitutions)
            }
            _ => (ty, vec![], HashMap::new()),
        }
    }

    fn check_type_args_count(&mut self, node: &ast::NamedType, ty: &types::GenericType) {
        match &node.args {
            Some(args) if args.len() > ty.params.len() => {
                let error = DiagnosticKind::TooManyParams {
                    expected: ty.params.len(),
                    got: args.len(),
                };
                self.error(error, node.loc);
            }
            _ => {}
        }
    }

    fn get_expected_fields(&mut self, ty: TypeId, loc: Location) -> Option<ExpectedFields> {
        match self.resolve(ty) {
            types::Type::Struct(st) => Some(ExpectedFields::Struct(st.fields)),
            types::Type::Tuple(tu) => Some(ExpectedFields::Tuple(tu.elements)),
            types::Type::Map(m) => Some(ExpectedFields::MapEntries(m)),
            types::Type::Unit => None,
            _ => {
                self.error(DiagnosticKind::InvalidTypeConstructor, loc);
                Some(ExpectedFields::Any)
            }
        }
    }

    fn get_variant_expected_fields(
        &mut self,
        variant: &ast::VariantConstructor,
        variant_ty: TypeId,
        parent_loc: Location,
    ) -> Option<ExpectedFields> {
        let types::Type::Enum(e) = self.resolve(variant_ty) else {
            self.error(DiagnosticKind::InvalidTypeConstructor, variant.loc);
            return Some(ExpectedFields::Tuple(vec![]));
        };

        let Some(variant_name) = &variant.variant_name else {
            return Some(ExpectedFields::Any);
        };

        let v = e.variants.iter().find(|v| v.name == variant_name.text);
        match v {
            Some(v) => self.get_expected_fields(v.def, parent_loc),
            None => {
                let error = DiagnosticKind::UnknownVariant {
                    variant: variant_name.text.clone(),
                    enum_name: variant.enum_name.name.clone(),
                };
                self.error(error, variant_name.loc);
                Some(ExpectedFields::Any)
            }
        }
    }

    fn check_literal_body(
        &mut self,
        body: &Option<ast::ConstructorBody>,
        expected: Option<ExpectedFields>,
        mut substitutions: &mut HashMap<types::TypeParam, TypeId>,
    ) {
        let Some(body) = body else { return };

        match expected {
            Some(ExpectedFields::MapEntries(m)) => match body {
                ast::ConstructorBody::Struct(st) => {
                    st.fields
                        .iter()
                        .for_each(|f| self.check_map_entry(f, &m, &mut substitutions));
                }
                ast::ConstructorBody::Tuple(t) => {
                    self.handle_unexpected_tuple_body(t, true);
                }
            },
            Some(ExpectedFields::Struct(expected_fields)) => match body {
                ast::ConstructorBody::Struct(st) => {
                    let mut encountered_field_names = HashSet::new();
                    for field in &st.fields {
                        self.check_struct_field(
                            field,
                            &expected_fields,
                            &mut encountered_field_names,
                            &mut substitutions,
                        );
                    }
                    expected_fields
                        .iter()
                        .filter(|f| !encountered_field_names.contains(&f.name))
                        .for_each(|f| {
                            let error = DiagnosticKind::MissingField {
                                name: f.name.clone(),
                            };
                            self.error(error, st.loc);
                        });
                }
                ast::ConstructorBody::Tuple(t) => {
                    self.handle_unexpected_tuple_body(t, true);
                }
            },
            Some(ExpectedFields::Tuple(expected_tuple)) => match body {
                ast::ConstructorBody::Struct(st) => {
                    self.handle_unexpected_struct_body(st, true);
                }
                ast::ConstructorBody::Tuple(t) => {
                    if expected_tuple.len() != t.elements.len() {
                        let error = DiagnosticKind::ArgumentCountMismatch {
                            expected: expected_tuple.len(),
                            got: t.elements.len(),
                        };
                        self.error(error, t.loc);
                    }
                    for (expected, got) in expected_tuple.iter().zip(t.elements.iter()) {
                        self.check_expression_against(got, *expected, &mut substitutions);
                    }
                }
            },
            _ => match body {
                ast::ConstructorBody::Struct(st) => {
                    self.handle_unexpected_struct_body(st, false);
                }
                ast::ConstructorBody::Tuple(t) => {
                    self.handle_unexpected_tuple_body(t, false);
                }
            },
        }
    }

    fn check_map_entry(
        &mut self,
        entry: &ast::ConstructorField,
        expected: &types::MapType,
        mut substitutions: &mut HashMap<types::TypeParam, TypeId>,
    ) {
        match &entry.key {
            Some(ast::ConstructorKey::MapKey(e)) => {
                self.check_expression_against(e, expected.key, &mut substitutions)
            }
            Some(ast::ConstructorKey::Name(n)) => {
                self.error(DiagnosticKind::ExpectedMapKey, n.loc);
            }
            None => {}
        }
        if let Some(expr) = &entry.value {
            self.check_expression_against(expr, expected.value, &mut substitutions);
        }
    }

    fn check_struct_field(
        &mut self,
        field: &ast::ConstructorField,
        expected_fields: &Vec<StructField>,
        encountered_field_names: &mut HashSet<String>,
        mut substitutions: &mut HashMap<types::TypeParam, TypeId>,
    ) {
        let Some(key) = &field.key else {
            field.value.as_ref().map(|v| self.visit_expression(v));
            return;
        };
        let ast::ConstructorKey::Name(n) = key else {
            self.error(DiagnosticKind::InvalidMember, field.loc);
            field.value.as_ref().map(|v| self.visit_expression(v));
            return;
        };

        let Some(expected_field) = expected_fields.iter().find(|f| f.name == n.text) else {
            let error = DiagnosticKind::UnknownMember {
                member: n.as_str().to_string(),
            };
            self.error(error, n.loc);
            return;
        };
        encountered_field_names.insert(n.as_str().to_string());
        if let Some(value) = &field.value {
            self.check_expression_against(value, expected_field.def, &mut substitutions);
        }
    }

    fn handle_unexpected_tuple_body(&mut self, t: &ast::TupleExpression, report: bool) {
        if report {
            self.error(DiagnosticKind::ExpectedStructLikeBody, t.loc);
        }
        t.elements.iter().for_each(|e| {
            self.visit_expression(e);
        });
    }

    fn handle_unexpected_struct_body(&mut self, st: &ast::StructLiteralBody, report: bool) {
        if report {
            self.error(DiagnosticKind::ExpectedTupleLikeBody, st.loc);
        }
        st.fields
            .iter()
            .filter_map(|f| f.value.as_ref())
            .for_each(|v| {
                self.visit_expression(v);
            })
    }

    fn resolve_constructor_type(
        &mut self,
        unresolved_type: TypeId,
        constructor_params: Vec<TypeId>,
        at: Location,
        substitutions: &HashMap<types::TypeParam, TypeId>,
    ) -> TypeId {
        let mut unresolved = false;
        let resolved_type_args = constructor_params
            .into_iter()
            .map(|p| match self.resolve(p) {
                types::Type::Param(param) => param,
                _ => panic!(),
            })
            .map(|p| match substitutions.get(&p) {
                Some(id) => *id,
                None => {
                    unresolved = true;
                    TypeStore::UNKNOWN
                }
            })
            .collect::<Vec<_>>();

        if unresolved {
            self.error(DiagnosticKind::CannotInferType, at);
        }

        self.session
            .types()
            .substitute(unresolved_type, &resolved_type_args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::session::Session;
    use crate::types::{MapType, StructField, Type, Variant};
    use crate::{ast, Location, SymbolData, SymbolKind};

    #[test]
    fn test_visit_map_literal() {
        let session = Session::new();
        let mut checker = TypeChecker::new(&session, 0);
        let map_literal = ast::ConstructorLiteral {
            loc: Location::dummy(),
            qualifiers: vec![],
            constructor: ast::Constructor::Map(ast::MapType {
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
            }),
            body: Some(ast::ConstructorBody::Struct(ast::StructLiteralBody {
                loc: Location::dummy(),
                fields: vec![ast::ConstructorField {
                    loc: Location::dummy(),
                    key: Some(ast::ConstructorKey::MapKey(ast::Expression::StringLiteral(
                        ast::StringLiteral {
                            loc: Location::dummy(),
                            text: "key".into(),
                        },
                    ))),
                    value: Some(ast::Expression::IntLiteral(ast::IntLiteral {
                        value: 42,
                        loc: Location::dummy(),
                    })),
                }],
            })),
        };

        let result = checker.visit_constructor_literal(&map_literal);
        let result = checker.resolve(result).clone();
        assert!(matches!(
            result,
            Type::Map(MapType {
                key: TypeStore::STRING,
                value: TypeStore::INTEGER
            })
        ));
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn test_visit_struct_literal() {
        let session = Session::new();
        let mut checker = TypeChecker::new(&session, 0);
        // Define the User struct type properly
        let user_type = types::Type::Struct(types::StructType {
            id: 0,
            fields: vec![
                StructField {
                    name: "name".to_string(),
                    def: TypeStore::STRING,
                    optional: false,
                },
                StructField {
                    name: "age".to_string(),
                    def: TypeStore::INTEGER,
                    optional: false,
                },
            ],
        });

        let user_type_id = checker.intern_unique(user_type);
        checker.ctx.register_symbol(SymbolData {
            name: "User".into(),
            ty: user_type_id,
            kind: SymbolKind::Type { members: vec![] },
            ..Default::default()
        });

        let struct_literal = ast::ConstructorLiteral {
            loc: Location::dummy(),
            qualifiers: vec![],
            constructor: ast::Constructor::Named(ast::NamedType {
                name: "User".to_string(),
                args: None,
                loc: Location::dummy(),
            }),
            body: Some(ast::ConstructorBody::Struct(ast::StructLiteralBody {
                loc: Location::dummy(),
                fields: vec![
                    ast::ConstructorField {
                        loc: Location::dummy(),
                        key: Some(ast::ConstructorKey::Name(ast::Identifier {
                            loc: Location::dummy(),
                            text: "name".to_string(),
                        })),
                        value: Some(ast::Expression::StringLiteral(ast::StringLiteral {
                            loc: Location::dummy(),
                            text: "John".into(),
                        })),
                    },
                    ast::ConstructorField {
                        loc: Location::dummy(),
                        key: Some(ast::ConstructorKey::Name(ast::Identifier {
                            loc: Location::dummy(),
                            text: "age".to_string(),
                        })),
                        value: Some(ast::Expression::IntLiteral(ast::IntLiteral {
                            loc: Location::dummy(),
                            value: 30,
                        })),
                    },
                ],
            })),
        };

        let result = checker.visit_constructor_literal(&struct_literal);
        let result_resolved = checker.resolve(result);

        assert!(matches!(
            result_resolved,
            types::Type::Struct(types::StructType { .. })
        ));
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn test_visit_variant_literal_valid() {
        let session = Session::new();
        let mut checker = TypeChecker::new(&session, 0);
        // Define a sum type with variants
        let enum_type = types::Type::Enum(types::EnumType {
            id: 0,
            variants: vec![
                Variant {
                    name: "Circle".to_string(),
                    def: checker.intern_unique(types::Type::Struct(types::StructType {
                        id: 0,
                        fields: vec![StructField {
                            name: "radius".to_string(),
                            def: TypeStore::INTEGER,
                            optional: false,
                        }],
                    })),
                },
                Variant {
                    name: "Rectangle".to_string(),
                    def: checker.intern_unique(types::Type::Struct(types::StructType {
                        id: 0,
                        fields: vec![
                            StructField {
                                name: "width".to_string(),
                                def: TypeStore::INTEGER,
                                optional: false,
                            },
                            StructField {
                                name: "height".to_string(),
                                def: TypeStore::INTEGER,
                                optional: false,
                            },
                        ],
                    })),
                },
            ],
        });

        let shape_type_id = checker.intern_unique(enum_type);
        checker.ctx.register_symbol(SymbolData {
            name: "Shape".into(),
            ty: shape_type_id,
            kind: SymbolKind::Type { members: vec![] },
            ..Default::default()
        });

        let variant_literal = ast::ConstructorLiteral {
            loc: Location::dummy(),
            qualifiers: vec![],
            constructor: ast::Constructor::Variant(ast::VariantConstructor {
                loc: Location::dummy(),
                enum_name: Box::new(ast::NamedType {
                    name: "Shape".to_string(),
                    args: None,
                    loc: Location::dummy(),
                }),
                variant_name: Some(ast::Identifier {
                    loc: Location::dummy(),
                    text: "Circle".to_string(),
                }),
            }),
            body: Some(ast::ConstructorBody::Struct(ast::StructLiteralBody {
                loc: Location::dummy(),
                fields: vec![ast::ConstructorField {
                    loc: Location::dummy(),
                    key: Some(ast::ConstructorKey::Name(ast::Identifier {
                        loc: Location::dummy(),
                        text: "radius".to_string(),
                    })),
                    value: Some(ast::Expression::IntLiteral(ast::IntLiteral {
                        value: 10,
                        loc: Location::dummy(),
                    })),
                }],
            })),
        };

        let result = checker.visit_constructor_literal(&variant_literal);
        let result = checker.resolve(result).clone();
        assert!(
            matches!(result, types::Type::Enum(types::EnumType { .. })),
            "expected enum type, got {:?}",
            result
        );
        assert!(
            checker.diagnostics.is_empty(),
            "expected no errors, got {:?}",
            checker.diagnostics
        );
    }
}
