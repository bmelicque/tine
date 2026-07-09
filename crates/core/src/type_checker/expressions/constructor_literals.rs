use core::panic;
use std::collections::HashSet;

use crate::{
    ast,
    diagnostics::DiagnosticKind,
    ir::{self, StructConstructor},
    type_checker::{
        substitutions::{SubstitutionTable, Substitutions},
        symbols::{EnumSymbolId, MemberSymbolId, SymbolId, TypeSymbolBody, TypeSymbolId},
        type_store::TypeStore,
    },
    types::{self, TypeId},
    Location,
};

use super::super::TypeChecker;

struct ConstructorVisit {
    pub constructor: StructConstructor,
    pub expected_body: Option<TypeSymbolBody>,
    pub substitutions: Substitutions,
}

impl TypeChecker {
    pub fn visit_constructor_literal(
        &mut self,
        node: ast::ConstructorLiteral,
    ) -> Option<ir::Expression> {
        match &node.constructor {
            ast::Constructor::Map(_) => {
                return self.visit_map_literal(node).map(Into::into);
            }
            ast::Constructor::Invalid(_) => {
                self.fallback_check_literal_body(node.body);
                return None;
            }
            _ => {}
        }

        let Some(ConstructorVisit {
            constructor,
            expected_body,
            substitutions,
        }) = self.visit_constructor(node.constructor)
        else {
            self.fallback_check_literal_body(node.body);
            return None;
        };

        match node.body {
            Some(ast::ConstructorBody::Struct(s)) => match expected_body {
                Some(TypeSymbolBody::Struct(fields)) => self
                    .visit_struct_literal_body(
                        s,
                        fields.into_iter().map(|f| f.1).collect(),
                        constructor,
                        substitutions,
                    )
                    .map(Into::into),
                _ => {
                    self.handle_unexpected_struct_body(s, true);
                    None
                }
            },
            Some(ast::ConstructorBody::Tuple(t)) => match expected_body {
                Some(TypeSymbolBody::Tuple(elements)) => self
                    .visit_struct_tuple_body(t, elements, constructor, substitutions)
                    .map(Into::into),
                _ => {
                    self.handle_unexpected_tuple_body(t, true);
                    None
                }
            },
            None => match expected_body {
                None => Some(ir::Expression::Array(ir::ArrayExpression {
                    loc: node.loc,
                    elements: vec![],
                    ty: self.constructor_type(&constructor),
                })),
                _ => {
                    self.error(DiagnosticKind::ExpectedVariantUnit, node.loc);
                    None
                }
            },
        }
    }

    /// Get the type's symbol and a list of expected members.
    ///
    /// Members are not directly in the type symbol because of enums (it is then in the appropriate constructor)
    fn visit_constructor(&mut self, node: ast::Constructor) -> Option<ConstructorVisit> {
        match node {
            ast::Constructor::Invalid(_) => None,
            ast::Constructor::Map(_) => None,
            ast::Constructor::Named(named) => self.visit_named_constructor(named),
            ast::Constructor::Variant(variant) => self.visit_variant_constructor(variant),
        }
    }

    fn visit_named_constructor(&mut self, named: ast::NamedType) -> Option<ConstructorVisit> {
        let symbol = self.find_type(&named)?;
        let expected_type_params = match self.symbol_type(symbol) {
            types::Type::Generic(g) => g.params,
            _ => vec![],
        };
        let (_, substitutions) = self.visit_type_args(named.args, &expected_type_params, named.loc);
        let symbol_id = match symbol {
            TypeSymbolId::Struct(st) => st,
            _ => {
                self.error(DiagnosticKind::ExpectedStructGotEnum, named.loc);
                return None;
            }
        };
        let body = self.symbols.get(symbol_id).body.clone();
        Some(ConstructorVisit {
            constructor: StructConstructor::Struct(named.loc, symbol_id.clone()),
            expected_body: Some(body),
            substitutions,
        })
    }

    /// Visit constructors of the form `Enum.Variant`
    fn visit_variant_constructor(
        &mut self,
        node: ast::VariantConstructor,
    ) -> Option<ConstructorVisit> {
        let (enum_id, type_params) = self.resolve_enum_name(&node.enum_name)?;
        let (_, substitutions) =
            self.visit_type_args(node.enum_name.args, &type_params, node.enum_name.loc);
        let enum_name = node.enum_name.name;
        let variant_name = node.variant_name?;
        let e = self.symbols.get(enum_id);
        let variant = e
            .variants
            .iter()
            .find(|v| self.symbol_name(**v) == variant_name.text)
            .copied();
        let Some(variant_id) = variant else {
            let error = DiagnosticKind::UnknownVariant {
                variant: variant_name.text,
                enum_name: enum_name.text,
            };
            self.error(error, variant_name.loc);
            return None;
        };
        let expected_body = self.symbols.get(variant_id).body.clone();
        Some(ConstructorVisit {
            constructor: StructConstructor::Enum(node.loc, enum_id, variant_id),
            expected_body,
            substitutions,
        })
    }

    fn find_type(&mut self, ty: &ast::NamedType) -> Option<TypeSymbolId> {
        let Some(symbol) = self.get_symbol_id(ty.name.as_str()) else {
            let error = DiagnosticKind::CannotFindName {
                name: ty.name.as_str().to_string(),
            };
            self.error(error, ty.loc);
            return None;
        };
        let symbol = symbol.as_type_symbol_id();
        if symbol.is_none() {
            self.error(DiagnosticKind::ExpectedTypeGotValue, ty.loc);
        }
        symbol
    }

    fn constructor_type(&self, constructor: &StructConstructor) -> TypeId {
        match constructor {
            StructConstructor::Enum(_, e, _) => self.symbols.get(*e).ty,
            StructConstructor::Struct(_, s) => self.symbols.get(*s).ty,
        }
    }

    /// Tries to resolve the name of the constructor being called.
    /// Returns the type definition and a substitution map.
    /// `GenericType` definitions will be unwrapped to the inner definition type.
    fn resolve_enum_name(
        &mut self,
        node: &ast::NamedType,
    ) -> Option<(EnumSymbolId, Vec<types::TypeParam>)> {
        let symbol = match self.get_symbol_id(node.name.as_str()) {
            Some(SymbolId::Enum(symbol)) => symbol,
            Some(_) => {
                self.error(DiagnosticKind::InvalidTypeConstructor, node.name.loc);
                return None;
            }
            None => {
                let error = DiagnosticKind::CannotFindName {
                    name: node.name.as_str().to_string(),
                };
                self.error(error, node.loc);
                return None;
            }
        };

        match self.symbol_type(symbol) {
            types::Type::Generic(g) => Some((symbol, g.params)),
            _ => Some((symbol, vec![])),
        }
    }

    fn visit_struct_literal_body(
        &mut self,
        body: ast::StructLiteralBody,
        members: Vec<MemberSymbolId>,
        constructor: StructConstructor,
        mut substitutions: Substitutions,
    ) -> Option<ir::StructLiteral> {
        let mut encountered = HashSet::new();
        let fields = body
            .fields
            .into_iter()
            .map(|field| {
                self.visit_struct_field(field, &members, &mut encountered, &mut substitutions)
            })
            .collect::<Vec<Option<_>>>()
            .into_iter()
            .collect::<Option<Vec<_>>>()?;
        let ty = self.resolve_constructor_type(
            self.constructor_type(&constructor),
            body.loc,
            &substitutions,
        );

        Some(ir::StructLiteral {
            loc: body.loc,
            constructor,
            fields,
            ty,
        })
    }

    fn visit_struct_field(
        &mut self,
        field: ast::ConstructorField,
        members: &[MemberSymbolId],
        encountered_field_names: &mut HashSet<String>,
        mut substitutions: &mut Substitutions,
    ) -> Option<ir::StructLiteralField> {
        let key = match field.key {
            Some(ast::ConstructorKey::Name(n)) => n,
            Some(_) => {
                self.error(DiagnosticKind::InvalidMember, field.loc);
                field.value.and_then(|v| self.visit_expression(v));
                return None;
            }
            None => {
                field.value.and_then(|v| self.visit_expression(v));
                return None;
            }
        };

        let Some(&symbol) = members.iter().find(|&&f| self.symbol_name(f) == key.text) else {
            let error = DiagnosticKind::UnknownMember {
                member: key.as_str().to_string(),
            };
            self.error(error, key.loc);
            return None;
        };
        if !self.is_visible(symbol.into()) {
            let error = DiagnosticKind::FieldIsPrivate(key.text.clone());
            self.error(error, field.loc);
        }
        encountered_field_names.insert(key.as_str().to_string());

        let value = field.value.and_then(|v| {
            self.check_expression_against(v, self.symbol_type_id(symbol), &mut substitutions)
        })?;

        Some(ir::StructLiteralField {
            loc: field.loc,
            name: (key.loc, symbol),
            value,
        })
    }

    /// Visit the body of tuple-like structs, like `Struct(a, b)`.
    ///
    /// These get lowered to regular structs `{ _0: a, _1: b }`.
    fn visit_struct_tuple_body(
        &mut self,
        body: ast::TupleExpression,
        members: Vec<MemberSymbolId>,
        constructor: StructConstructor,
        mut substitutions: Substitutions,
    ) -> Option<ir::StructLiteral> {
        let fields = body
            .elements
            .into_iter()
            .zip(members.into_iter())
            .map(|(got, expected)| {
                self.visit_struct_tuple_element(got, expected, &mut substitutions)
            })
            .collect::<Vec<Option<_>>>()
            .into_iter()
            .collect::<Option<Vec<_>>>()?;
        let ty = self.resolve_constructor_type(
            self.constructor_type(&constructor),
            body.loc,
            &substitutions,
        );
        Some(ir::StructLiteral {
            loc: body.loc,
            constructor,
            fields,
            ty,
        })
    }

    fn visit_struct_tuple_element(
        &mut self,
        got: ast::Expression,
        expected: MemberSymbolId,
        mut substitutions: &mut Substitutions,
    ) -> Option<ir::StructLiteralField> {
        let ty = self.symbol_type_id(expected);
        let expr = self.check_expression_against(got, ty, &mut substitutions)?;
        Some(ir::StructLiteralField {
            loc: expr.loc(),
            name: (expr.loc(), expected),
            value: expr,
        })
    }

    fn resolve_constructor_type(
        &mut self,
        unresolved_type: TypeId,
        at: Location,
        substitutions: &Substitutions,
    ) -> TypeId {
        let generic = match self.resolve(unresolved_type) {
            types::Type::Generic(g) => g,
            _ => return unresolved_type,
        };

        let table: SubstitutionTable = substitutions.into();
        let mut unresolved = false;
        let args = generic
            .params
            .iter()
            .map(|p| match table.get(p) {
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

        self.intern(types::TypeRef {
            inner: unresolved_type,
            args,
        })
    }

    fn visit_map_literal(&mut self, node: ast::ConstructorLiteral) -> Option<ir::MapLiteral> {
        let ast::Constructor::Map(constructor) = node.constructor else {
            panic!()
        };
        let ty = self.visit_map_type(constructor);

        let entries = match node.body {
            Some(ast::ConstructorBody::Struct(body)) => body
                .fields
                .into_iter()
                .filter_map(|entry| self.visit_map_entry(entry))
                .collect::<Vec<_>>(),
            Some(ast::ConstructorBody::Tuple(body)) => {
                self.handle_unexpected_tuple_body(body, true);
                return None;
            }
            None => {
                self.error(DiagnosticKind::ExpectedStructLikeBody, node.loc);
                return None;
            }
        };

        let (key, value) = self.validate_map_type(ty, &entries);
        match key {
            TypeStore::DYNAMIC => self.error(DiagnosticKind::CannotInferType, node.loc),
            TypeStore::UNKNOWN
            | TypeStore::BOOLEAN
            | TypeStore::FLOAT
            | TypeStore::INTEGER
            | TypeStore::STRING => {}
            _ => self.error(DiagnosticKind::NotImplementedMapType, node.loc),
        }
        let ty = self.intern(types::MapType { key, value });
        Some(ir::MapLiteral {
            loc: node.loc,
            entries,
            ty,
        })
    }

    fn visit_map_entry(&mut self, entry: ast::ConstructorField) -> Option<ir::MapEntry> {
        let key = match entry.key {
            Some(ast::ConstructorKey::MapKey(e)) => self.visit_expression(e),
            Some(ast::ConstructorKey::Name(n)) => {
                self.error(DiagnosticKind::ExpectedMapKey, n.loc);
                None
            }
            None => None,
        };
        let value = entry.value.and_then(|v| self.visit_expression(v));
        Some(ir::MapEntry {
            loc: entry.loc,
            key: key?,
            value: value?,
        })
    }

    fn validate_map_type(
        &mut self,
        expected: TypeId,
        entries: &[ir::MapEntry],
    ) -> (TypeId, TypeId) {
        let types::Type::Map(map_type) = self.resolve(expected) else {
            panic!()
        };
        let mut expected_key_type = map_type.key;
        let mut expected_value_type = map_type.value;
        for entry in entries {
            expected_key_type =
                self.check_entry_part_type(expected_key_type, entry.key.ty(), entry.key.loc());
            expected_value_type = self.check_entry_part_type(
                expected_value_type,
                entry.value.ty(),
                entry.value.loc(),
            );
        }

        (expected_key_type, expected_value_type)
    }

    // Return new expected type
    fn check_entry_part_type(&mut self, expected: TypeId, got: TypeId, at: Location) -> TypeId {
        if expected == TypeStore::DYNAMIC {
            if got != TypeStore::UNKNOWN {
                return got;
            }
        } else {
            self.check_assigned_type(expected, got, true, at);
        }
        return expected;
    }

    fn handle_unexpected_tuple_body(&mut self, t: ast::TupleExpression, report: bool) {
        if report {
            self.error(DiagnosticKind::ExpectedStructLikeBody, t.loc);
        }
        t.elements.into_iter().for_each(|e| {
            self.visit_expression(e);
        });
    }

    fn handle_unexpected_struct_body(&mut self, st: ast::StructLiteralBody, report: bool) {
        if report {
            self.error(DiagnosticKind::ExpectedTupleLikeBody, st.loc);
        }
        st.fields.into_iter().filter_map(|f| f.value).for_each(|v| {
            self.visit_expression(v);
        })
    }

    fn fallback_check_literal_body(&mut self, body: Option<ast::ConstructorBody>) {
        match body {
            Some(ast::ConstructorBody::Struct(st)) => self.handle_unexpected_struct_body(st, false),
            Some(ast::ConstructorBody::Tuple(t)) => self.handle_unexpected_tuple_body(t, false),
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MapType, Type};
    use crate::{ast, Location};

    #[test]
    fn test_visit_map_literal() {
        let mut checker = TypeChecker::new();
        let map_literal = ast::ConstructorLiteral {
            loc: Location::dummy(),
            qualifiers: vec![],
            constructor: ast::Constructor::Map(ast::MapType {
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

        let result = checker.visit_constructor_literal(map_literal);
        let result = checker.resolve(result.unwrap().ty());
        assert!(matches!(
            result,
            Type::Map(MapType {
                key: TypeStore::STRING,
                value: TypeStore::INTEGER
            })
        ));
        assert!(checker.diagnostics.is_empty());
    }
}
