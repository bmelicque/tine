use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};
use tine_ir::{self as ir, Typed};
use tine_symbols::symbols::*;
use tine_types::{store::TypeStore, types};

use crate::{substitutions::Substitutions, TypeChecker};

impl TypeChecker {
    pub(super) fn visit_map_literal(
        &mut self,
        node: ast::ConstructorLiteral,
    ) -> Option<ir::Expression> {
        let ast::Constructor::Map(constructor) = node.constructor else {
            panic!()
        };
        let ty = self.visit_map_type(constructor);
        // Unwrap should be safe because `visit_map_type` interns a `TypeRef`
        let entries = self.visit_map_entries(node.body, node.loc)?;
        let type_ = self.types.get(ty).as_ref().unwrap();
        let (key, value) = self.validate_map_type((type_.args[0], type_.args[1]), &entries);
        self.validate_key_type(key, node.loc);

        let map_symbol_id = self.builtin_id::<StructSymbolId>("Map").unwrap();
        let ty = self.intern(types::TypeRef {
            inner: self.symbol_type_id(map_symbol_id),
            args: vec![key, value],
        });
        let empty = ir::Expression::IntrinsicConstruct(ir::IntrinsicConstruct {
            ty,
            loc: node.loc,
            constructor: map_symbol_id,
            fields: vec![],
        });
        if entries.is_empty() {
            return Some(empty);
        }

        let (tmp, symbol_id) = self.make_temp_variable(node.loc, &empty);
        let mut stmts = vec![ir::Statement::Variable(ir::VariableDeclaration {
            loc: node.loc,
            mutable: true,
            symbol: symbol_id,
            value: empty,
        })];

        let (insert, _) = self.get_insert_method(map_symbol_id, key, value);

        for entry in entries {
            let method = ir::Expression::Method(ir::MethodExpression {
                loc: node.loc,
                ty: TypeStore::BOOLEAN,
                host: Box::new(tmp.clone().into()),
                method: (node.loc, insert),
                args: entry.into(),
            });
            stmts.push(method.into())
        }

        let ty = tmp.ty;
        stmts.push(ir::Statement::Expression(tmp.into()));

        Some(ir::Expression::Block(ir::Block {
            ty,
            loc: node.loc,
            statements: stmts,
        }))
    }

    fn visit_map_entries(
        &mut self,
        body: Option<ast::ConstructorBody>,
        loc: Location,
    ) -> Option<Vec<[ir::Expression; 2]>> {
        match body {
            Some(ast::ConstructorBody::Struct(body)) => body
                .fields
                .into_iter()
                .filter_map(|entry| self.visit_map_entry(entry))
                .collect::<Vec<_>>()
                .into(),
            Some(ast::ConstructorBody::Tuple(body)) => {
                self.handle_unexpected_tuple_body(body, true);
                None
            }
            None => {
                self.error(DiagnosticKind::ExpectedStructLikeBody, loc);
                None
            }
        }
    }

    fn visit_map_entry(&mut self, entry: ast::ConstructorField) -> Option<[ir::Expression; 2]> {
        let key = match entry.key {
            Some(ast::ConstructorKey::MapKey(e)) => self.visit_expression(e),
            Some(ast::ConstructorKey::Name(n)) => {
                self.error(DiagnosticKind::ExpectedMapKey, n.loc);
                None
            }
            None => None,
        };
        let value = entry.value.and_then(|v| self.visit_expression(v));
        Some([key?, value?])
    }

    fn validate_map_type(
        &mut self,
        expected: (types::TypeId, types::TypeId),
        entries: &[[ir::Expression; 2]],
    ) -> (types::TypeId, types::TypeId) {
        let mut expected_key_type = expected.0;
        let mut expected_value_type = expected.1;
        for entry in entries {
            expected_key_type =
                self.check_entry_part_type(expected_key_type, entry[0].ty(), entry[0].loc());
            expected_value_type =
                self.check_entry_part_type(expected_value_type, entry[1].ty(), entry[1].loc());
        }

        (expected_key_type, expected_value_type)
    }

    fn validate_key_type(&mut self, key: types::TypeId, loc: Location) {
        match key {
            TypeStore::DYNAMIC => {
                self.error(DiagnosticKind::CannotInferType, loc);
                return;
            }
            TypeStore::UNKNOWN => return,
            _ => {}
        }

        let hash_trait = self.resolve(self.builtin_symbol("Hash").unwrap().ty());
        let hash_trait = hash_trait.as_trait().unwrap();
        if !self.implements_trait(key, hash_trait, true) {
            let diag = DiagnosticKind::TypeDoesNotImplementTrait {
                type_name: self.types.display(key),
                trait_name: "Hash".to_string(),
            };
            self.error(diag, loc);
        }

        let eq_trait = self.resolve(self.builtin_symbol("Eq").unwrap().ty());
        let eq_trait = eq_trait.as_trait().unwrap();
        if !self.implements_trait(key, eq_trait, true) {
            let diag = DiagnosticKind::TypeDoesNotImplementTrait {
                type_name: self.types.display(key),
                trait_name: "Eq".to_string(),
            };
            self.error(diag, loc);
        }
    }

    // Return new expected type
    fn check_entry_part_type(
        &mut self,
        expected: types::TypeId,
        got: types::TypeId,
        at: Location,
    ) -> types::TypeId {
        if expected == TypeStore::DYNAMIC {
            if got != TypeStore::UNKNOWN {
                return got;
            }
        } else {
            self.check_assigned_type(expected, got, true, at);
        }
        return expected;
    }

    fn get_insert_method(
        &mut self,
        map_id: StructSymbolId,
        key: types::TypeId,
        value: types::TypeId,
    ) -> (MethodSymbolId, types::TypeId) {
        let insert = *self
            .symbol_methods(map_id)
            .iter()
            .find(|s| self.symbol_name(**s) == "insert")
            .unwrap();
        let generic_map_ty = self.resolve(self.symbols.get(map_id).ty);
        let map_params = &generic_map_ty.as_struct().as_ref().unwrap().params;
        let subs = Substitutions::with_initial(&map_params, &[key, value]);
        let generic_insert_ty = self.symbol_type_id(insert);
        let concrete_ty = subs.apply(&mut self.types, generic_insert_ty);
        (insert, concrete_ty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        match result {
            types::Type::Ref(r) => assert_eq!(r.args, vec![TypeStore::STRING, TypeStore::INTEGER]),
            _ => panic!("Expected a ref type"),
        }
        assert!(
            checker.diagnostics.is_empty(),
            "expected no diagnostics: {:?}",
            checker.diagnostics
        );
    }
}
