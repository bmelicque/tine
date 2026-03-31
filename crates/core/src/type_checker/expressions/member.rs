use crate::{
    ast, ir,
    type_checker::{
        analysis_context::{symbols::TypeSymbolBody, type_store::TypeStore},
        TypeChecker,
    },
    types::Type,
    DiagnosticKind, SymbolKind,
};

impl TypeChecker<'_> {
    pub fn visit_member_expression(
        &mut self,
        expr: ast::MemberExpression,
    ) -> Option<ir::MemberExpression> {
        let Some(member) = &expr.prop else {
            expr.object.and_then(|o| self.visit_expression(*o));
            // missing member already reported during parsing phase
            return None;
        };
        match member {
            ast::MemberProp::FieldName(_) => self.visit_field_access(expr),
            ast::MemberProp::Index(_) => self.visit_tuple_indexing(expr),
        }
    }

    fn visit_field_access(&mut self, expr: ast::MemberExpression) -> Option<ir::MemberExpression> {
        debug_assert!(matches!(expr.prop, Some(ast::MemberProp::FieldName(_))));
        let object = expr.object.and_then(|o| self.visit_expression(*o))?;
        let Some(ast::MemberProp::FieldName(field_name)) = expr.prop else {
            unreachable!()
        };
        let Some(root_symbol) = self.resolve_type_symbol(object.ty()) else {
            let error = DiagnosticKind::UnknownMember {
                member: field_name.as_str().to_string(),
            };
            self.error(error, field_name.loc);
            return None;
        };

        let SymbolKind::Struct {
            body: TypeSymbolBody::Struct(fields),
            methods,
        } = &root_symbol.borrow().kind
        else {
            panic!();
        };

        let member = fields
            .get(field_name.as_str())
            .or_else(|| methods.iter().find(|m| m.borrow().name == field_name.text))
            .cloned();
        let member = match member {
            Some(symbol) => ir::Identifier {
                loc: field_name.loc,
                symbol,
            },
            None => {
                let error = DiagnosticKind::UnknownMember {
                    member: field_name.as_str().to_string(),
                };
                self.error(error, field_name.loc);
                return None;
            }
        };

        Some(ir::MemberExpression {
            loc: expr.loc,
            object: Box::new(object),
            // FIXME: handle substitutions for generics
            ty: member.ty(),
            member,
        })
    }

    pub fn visit_tuple_indexing(
        &mut self,
        expr: ast::MemberExpression,
    ) -> Option<ir::MemberExpression> {
        let Some(ast::MemberProp::Index(index)) = &expr.prop else {
            panic!();
        };

        // check object
        let object = expr.object.and_then(|o| self.visit_expression(*o))?;
        let Some(root_symbol) = self.resolve_type_symbol(object.ty()) else {
            let error = DiagnosticKind::UnknownMember {
                member: index.value.to_string(),
            };
            self.error(error, index.loc);
            return None;
        };

        let Type::Tuple(ty) = self.resolve(object.ty()) else {
            if object.ty() != TypeStore::UNKNOWN {
                let error = DiagnosticKind::ExpectedTuple {
                    got: self.session.display_type(object.ty()),
                };
                self.error(error, object.loc());
            }
            return None;
        };
        let SymbolKind::Struct {
            body: TypeSymbolBody::Tuple(elements),
            ..
        } = &root_symbol.borrow().kind
        else {
            panic!();
        };

        // check index is in range
        let value = index.value;
        if value < 0 {
            self.error(DiagnosticKind::NegativeTupleIndex, index.loc);
            return None;
        }
        let value = value as usize;
        if value >= elements.len() {
            self.error(
                DiagnosticKind::UnknownMember {
                    member: value.to_string(),
                },
                index.loc,
            );
            return None;
        }

        let member = ir::Identifier {
            loc: index.loc,
            symbol: elements[value].clone(),
        };

        Some(ir::MemberExpression {
            loc: expr.loc,
            object: Box::new(object),
            ty: ty.elements[value],
            member,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::{
        analyzer::session::Session,
        ast,
        locations::Span,
        type_checker::{analysis_context::symbols::TypeSymbolBody, test_utils::MockLoader},
        types::*,
        Location, SymbolData, SymbolKind,
    };

    fn create_type_checker() -> TypeChecker<'static> {
        let session = Box::leak(Box::new(Session::new(Box::new(MockLoader))));
        TypeChecker::new(session, 0)
    }

    fn loc(text: &'static str) -> Location {
        let span = Span::new(0, text.len() as u32);
        Location::new(0, span)
    }

    fn ident(text: &str) -> ast::Identifier {
        ast::Identifier {
            loc: Location::new(0, Span::new(0, text.len() as u32)),
            text: text.to_string(),
        }
    }

    #[test]
    fn test_visit_field_access_expression() {
        let mut checker = create_type_checker();
        let id = checker.intern_unique(Type::Struct(StructType {
            id: 0,
            fields: vec![
                StructField {
                    name: "name".to_string(),
                    def: TypeStore::STRING,
                },
                StructField {
                    name: "age".to_string(),
                    def: TypeStore::INTEGER,
                },
            ],
        }));
        checker.ctx.register_symbol(SymbolData {
            name: "User".into(),
            ty: id,
            kind: SymbolKind::Struct {
                body: TypeSymbolBody::Struct(HashMap::new()),
                methods: vec![],
            },
            ..Default::default()
        });

        let field_access_expression = ast::MemberExpression {
            object: Some(Box::new(ast::Expression::Identifier(ident("user")))),
            prop: Some(ident("name").into()),
            loc: Location::dummy(),
        };
        checker.ctx.register_symbol(SymbolData {
            name: "user".into(),
            ty: id,
            kind: SymbolKind::constant(),
            ..Default::default()
        });

        let result = checker
            .visit_member_expression(field_access_expression)
            .unwrap();
        assert!(
            checker.diagnostics.is_empty(),
            "Expected no errors, got {:?}",
            checker.diagnostics
        );
        assert_eq!(result.ty, TypeStore::STRING);
    }

    #[test]
    fn test_visit_tuple_indexing_valid() {
        let mut checker = create_type_checker();
        let tuple_type = Type::Tuple(TupleType {
            elements: vec![TypeStore::INTEGER, TypeStore::STRING, TypeStore::BOOLEAN],
        });

        let ty = checker.intern(tuple_type);
        checker.ctx.register_symbol(SymbolData {
            name: "my_tuple".into(),
            ty,
            kind: SymbolKind::constant(),
            defined_at: loc("my_tuple"),
            ..Default::default()
        });

        let tuple_indexing = ast::MemberExpression {
            object: Some(Box::new(ast::Expression::Identifier(ident("my_tuple")))),
            prop: Some(ast::MemberProp::Index(ast::IntLiteral {
                value: 1,
                loc: Location::dummy(),
            })),
            loc: Location::dummy(),
        };

        let result = checker.visit_tuple_indexing(tuple_indexing).unwrap();
        assert_eq!(result.ty, TypeStore::STRING);
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn test_visit_tuple_indexing_invalid_type() {
        let mut checker = create_type_checker();
        checker.ctx.register_symbol(SymbolData {
            name: "not_a_tuple".into(),
            ty: TypeStore::INTEGER,
            kind: SymbolKind::constant(),
            defined_at: loc("not_a_tuple"),
            ..Default::default()
        });

        let tuple_indexing = ast::MemberExpression {
            object: Some(Box::new(ast::Expression::Identifier(ident("not_a_tuple")))),
            prop: Some(ast::MemberProp::Index(ast::IntLiteral {
                value: 0,
                loc: Location::dummy(),
            })),
            loc: Location::dummy(),
        };

        let result = checker.visit_tuple_indexing(tuple_indexing).unwrap();
        assert_eq!(result.ty, TypeStore::UNKNOWN);
        assert_eq!(checker.diagnostics.len(), 1);
    }

    #[test]
    fn test_visit_tuple_indexing_out_of_range() {
        let mut checker = create_type_checker();
        let tuple_type = Type::Tuple(TupleType {
            elements: vec![TypeStore::INTEGER, TypeStore::STRING],
        });
        let tuple_type = checker.intern(tuple_type);
        checker.ctx.register_symbol(SymbolData {
            name: "my_tuple".into(),
            ty: tuple_type,
            kind: SymbolKind::constant(),
            defined_at: loc("my_tuple"),
            ..Default::default()
        });

        let tuple_indexing = ast::MemberExpression {
            object: Some(Box::new(ast::Expression::Identifier(ident("my_tuple")))),
            prop: Some(ast::MemberProp::Index(ast::IntLiteral {
                value: 2,
                loc: Location::dummy(),
            })),
            loc: Location::dummy(),
        };

        let result = checker.visit_tuple_indexing(tuple_indexing).unwrap();
        assert_eq!(result.ty, TypeStore::UNKNOWN);
        assert_eq!(checker.diagnostics.len(), 1);
        assert!(matches!(
            checker.diagnostics[0].kind,
            DiagnosticKind::UnknownMember { .. }
        ));
    }

    #[test]
    fn test_visit_tuple_indexing_negative_index() {
        let mut checker = create_type_checker();
        let tuple_type = Type::Tuple(TupleType {
            elements: vec![TypeStore::INTEGER, TypeStore::STRING],
        });
        let tuple_type = checker.intern(tuple_type);
        checker.ctx.register_symbol(SymbolData {
            name: "my_tuple".into(),
            ty: tuple_type,
            kind: SymbolKind::constant(),
            defined_at: loc("my_tuple"),
            ..Default::default()
        });

        let tuple_indexing = ast::MemberExpression {
            object: Some(Box::new(ast::Expression::Identifier(ident("my_tuple")))),
            prop: Some(ast::MemberProp::Index(ast::IntLiteral {
                value: -1,
                loc: Location::dummy(),
            })),
            loc: Location::dummy(),
        };

        let result = checker.visit_tuple_indexing(tuple_indexing).unwrap();
        assert_eq!(result.ty, TypeStore::UNKNOWN);
        assert_eq!(checker.diagnostics.len(), 1);
    }
}
