use core::panic;
use std::collections::{HashMap, HashSet};

use crate::{
    ast,
    diagnostics::DiagnosticKind,
    ir,
    type_checker::analysis_context::{symbols::TypeSymbolBody, type_store::TypeStore},
    types::{self, TypeId},
    Location, SymbolKind, SymbolRef,
};

use super::super::TypeChecker;

struct ConstructorVisit {
    pub type_symbol: SymbolRef,
    pub variant_symbol: Option<SymbolRef>,
    pub expected_body: Option<TypeSymbolBody>,
    pub substitutions: HashMap<types::TypeParam, TypeId>,
}

impl TypeChecker<'_> {
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
            type_symbol: constructor_symbol,
            variant_symbol,
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
                        constructor_symbol,
                        variant_symbol,
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
                    .visit_struct_tuple_body(
                        t,
                        elements,
                        constructor_symbol,
                        variant_symbol,
                        substitutions,
                    )
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
                    ty: constructor_symbol.borrow().ty,
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
            ast::Constructor::Named(named) => {
                let symbol = self.find_type(&named)?;
                let expected_type_params = match self.resolve(symbol.borrow().ty) {
                    types::Type::Generic(g) => g.params,
                    _ => vec![],
                };
                let (_, substitutions) =
                    self.visit_type_args(named.args, &expected_type_params, named.loc);
                let body = match &symbol.borrow().kind {
                    SymbolKind::Struct { body, .. } => body.clone(),
                    SymbolKind::Enum { .. } => {
                        self.error(DiagnosticKind::ExpectedStructGotEnum, named.loc);
                        return None;
                    }
                    _ => panic!(),
                };
                Some(ConstructorVisit {
                    type_symbol: symbol,
                    variant_symbol: None,
                    expected_body: Some(body),
                    substitutions,
                })
            }
            ast::Constructor::Variant(variant) => {
                let (symbol, type_params) = self.resolve_constructor_name(&variant.enum_name)?;
                let (_, substitutions) = self.visit_type_args(
                    variant.enum_name.args,
                    &type_params,
                    variant.enum_name.loc,
                );
                let SymbolKind::Enum { variants, .. } = &symbol.borrow().kind else {
                    self.error(DiagnosticKind::InvalidTypeConstructor, variant.loc);
                    return None;
                };
                let enum_name = variant.enum_name.name;
                let variant_name = variant.variant_name?;
                let variant = variants
                    .iter()
                    .find(|constructor| constructor.borrow().name == variant_name.text)
                    .cloned();
                let Some(constructor) = variant else {
                    let error = DiagnosticKind::UnknownVariant {
                        variant: variant_name.text,
                        enum_name: enum_name,
                    };
                    self.error(error, variant_name.loc);
                    return None;
                };
                let SymbolKind::Constructor { body, .. } = &constructor.borrow().kind else {
                    let error = DiagnosticKind::UnknownVariant {
                        variant: variant_name.text,
                        enum_name: enum_name,
                    };
                    self.error(error, variant_name.loc);
                    return None;
                };
                Some(ConstructorVisit {
                    type_symbol: symbol.clone(),
                    variant_symbol: Some(constructor.clone()),
                    expected_body: body.clone(),
                    substitutions,
                })
            }
        }
    }

    fn find_type(&mut self, ty: &ast::NamedType) -> Option<SymbolRef> {
        let Some(symbol) = self.lookup(&ty.name) else {
            let error = DiagnosticKind::CannotFindName {
                name: ty.name.clone(),
            };
            self.error(error, ty.loc);
            return None;
        };
        if !symbol.borrow().is_type_symbol() {
            self.error(DiagnosticKind::ExpectedTypeGotValue, ty.loc);
            return None;
        }
        Some(symbol)
    }

    /// Tries to resolve the name of the constructor being called.
    /// Returns the type definition and a substitution map.
    /// `GenericType` definitions will be unwrapped to the inner definition type.
    fn resolve_constructor_name(
        &mut self,
        name: &ast::NamedType,
    ) -> Option<(SymbolRef, Vec<TypeId>)> {
        let Some(symbol) = self.lookup(&name.name) else {
            let error = DiagnosticKind::CannotFindName {
                name: name.name.clone(),
            };
            self.error(error, name.loc);
            return None;
        };

        let ty = symbol.borrow().ty;
        match self.resolve(ty) {
            types::Type::Generic(g) => Some((symbol, g.params)),
            _ => Some((symbol, vec![])),
        }
    }

    fn visit_struct_literal_body(
        &mut self,
        body: ast::StructLiteralBody,
        members: Vec<SymbolRef>,
        constructor: SymbolRef,
        variant: Option<SymbolRef>,
        mut substitutions: HashMap<types::TypeParam, TypeId>,
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
        let ty = self.resolve_constructor_type(constructor.as_type(), body.loc, &substitutions);

        Some(ir::StructLiteral {
            loc: body.loc,
            constructor,
            variant,
            fields,
            ty,
        })
    }

    fn visit_struct_field(
        &mut self,
        field: ast::ConstructorField,
        members: &[SymbolRef],
        encountered_field_names: &mut HashSet<String>,
        mut substitutions: &mut HashMap<types::TypeParam, TypeId>,
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

        let Some(symbol) = members
            .iter()
            .find(|f| f.borrow().name == key.text)
            .cloned()
        else {
            let error = DiagnosticKind::UnknownMember {
                member: key.as_str().to_string(),
            };
            self.error(error, key.loc);
            return None;
        };
        encountered_field_names.insert(key.as_str().to_string());

        let value = field.value.and_then(|v| {
            self.check_expression_against(v, symbol.borrow().ty, &mut substitutions)
        })?;

        Some(ir::StructLiteralField {
            loc: field.loc,
            name: ir::Identifier {
                loc: key.loc,
                symbol,
            },
            value,
        })
    }

    /// Visit the body of tuple-like structs, like `Struct(a, b)`.
    ///
    /// These get lowered to regular structs `{ _0: a, _1: b }`.
    fn visit_struct_tuple_body(
        &mut self,
        body: ast::TupleExpression,
        members: Vec<SymbolRef>,
        constructor_symbol: SymbolRef,
        variant_symbol: Option<SymbolRef>,
        mut substitutions: HashMap<types::TypeParam, TypeId>,
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
        let ty =
            self.resolve_constructor_type(constructor_symbol.as_type(), body.loc, &substitutions);
        Some(ir::StructLiteral {
            loc: body.loc,
            constructor: constructor_symbol,
            variant: variant_symbol,
            fields,
            ty,
        })
    }

    fn visit_struct_tuple_element(
        &mut self,
        got: ast::Expression,
        expected: SymbolRef,
        mut substitutions: &mut HashMap<types::TypeParam, TypeId>,
    ) -> Option<ir::StructLiteralField> {
        let expr = self.check_expression_against(got, expected.borrow().ty, &mut substitutions)?;
        Some(ir::StructLiteralField {
            loc: expr.loc(),
            name: ir::Identifier {
                loc: expr.loc(),
                symbol: expected.clone(),
            },
            value: expr,
        })
    }

    fn resolve_constructor_type(
        &mut self,
        unresolved_type: TypeId,
        at: Location,
        substitutions: &HashMap<types::TypeParam, TypeId>,
    ) -> TypeId {
        let generic = match self.resolve(unresolved_type) {
            types::Type::Generic(g) => g,
            _ => return unresolved_type,
        };

        let mut unresolved = false;
        let args = generic
            .params
            .iter()
            .map(|p| match self.resolve(*p) {
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

        self.intern(types::GenericType {
            params: args,
            definition: generic.definition,
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
            self.check_assigned_type(expected, got, at);
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
    use crate::analyzer::session::Session;
    use crate::type_checker::test_utils::MockLoader;
    use crate::types::{MapType, Type};
    use crate::{ast, Location};

    #[test]
    fn test_visit_map_literal() {
        let session = Session::new(Box::new(MockLoader));
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
