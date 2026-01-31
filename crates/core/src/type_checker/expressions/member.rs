use crate::{
    ast,
    type_checker::{analysis_context::type_store::TypeStore, TypeChecker},
    types::{Type, TypeId},
    DiagnosticKind,
};

impl TypeChecker<'_> {
    pub fn visit_member_expression(&mut self, expr: &ast::MemberExpression) -> TypeId {
        let Some(ref member) = expr.prop else {
            self.visit_expression(&expr.object);
            // missing member already reported during parsing phase
            return TypeStore::UNKNOWN;
        };
        match member {
            ast::MemberProp::FieldName(_) => self.visit_field_access(expr),
            ast::MemberProp::Index(_) => self.visit_tuple_indexing(expr),
        }
    }

    fn visit_field_access(&mut self, expr: &ast::MemberExpression) -> TypeId {
        let root_type = self.visit_expression(&expr.object);
        let Some(ast::MemberProp::FieldName(ref field_name)) = expr.prop else {
            unreachable!()
        };

        let ty: Option<TypeId> = match self.resolve(root_type) {
            Type::Integer => self
                .session
                .find_method(field_name.as_str(), root_type)
                .map(|s| s.borrow().ty),
            Type::Float => self
                .session
                .find_method(field_name.as_str(), root_type)
                .map(|s| s.borrow().ty),
            Type::Struct(ref ty) => {
                let prop = field_name.as_str();
                ty.fields.iter().find(|f| f.name == prop).map(|f| f.def)
            }
            _ => None,
        };

        let ty = match ty {
            Some(ty) => ty,
            None => {
                let error = DiagnosticKind::UnknownMember {
                    member: field_name.as_str().to_string(),
                };
                self.error(error, expr.loc);
                TypeStore::UNKNOWN
            }
        };

        self.save_member_type(expr, ty)
    }

    pub fn visit_tuple_indexing(&mut self, expr: &ast::MemberExpression) -> TypeId {
        let root_type = self.visit_expression(&expr.object);
        let Type::Tuple(tuple) = self.resolve(root_type) else {
            let error = DiagnosticKind::ExpectedTuple {
                got: self.session.display_type(root_type),
            };
            self.error(error, expr.object.loc());
            return self.save_member_type(expr, TypeStore::UNKNOWN);
        };

        let Some(ast::MemberProp::Index(index)) = &expr.prop else {
            panic!();
        };
        let value = index.value;
        if value < 0 {
            self.error(DiagnosticKind::NegativeTupleIndex, index.loc);
            return self.save_member_type(expr, TypeStore::UNKNOWN);
        }
        let value = value as usize;
        if value >= tuple.elements.len() {
            self.error(
                DiagnosticKind::UnknownMember {
                    member: value.to_string(),
                },
                index.loc,
            );
            return self.save_member_type(expr, TypeStore::UNKNOWN);
        } else {
            return self.save_member_type(expr, tuple.elements[value]);
        }
    }

    fn save_member_type(&mut self, expr: &ast::MemberExpression, ty: TypeId) -> TypeId {
        self.ctx.save_expression_type(expr.loc, ty);
        ty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analyzer::session::Session, ast, locations::Span, types::*, Location, SymbolData,
        SymbolKind,
    };

    fn create_type_checker() -> TypeChecker<'static> {
        let session = Box::leak(Box::new(Session::new()));
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
                    optional: false,
                },
                StructField {
                    name: "age".to_string(),
                    def: TypeStore::INTEGER,
                    optional: false,
                },
            ],
        }));
        checker.ctx.register_symbol(SymbolData {
            name: "User".into(),
            ty: id,
            kind: SymbolKind::Type { members: vec![] },
            ..Default::default()
        });

        let field_access_expression = ast::MemberExpression {
            object: Box::new(ast::Expression::Identifier(ident("user"))),
            prop: Some(ident("name").into()),
            loc: Location::dummy(),
        };
        checker.ctx.register_symbol(SymbolData {
            name: "user".into(),
            ty: id,
            kind: SymbolKind::constant(),
            ..Default::default()
        });

        let result = checker.visit_member_expression(&field_access_expression);
        assert!(
            checker.diagnostics.is_empty(),
            "Expected no errors, got {:?}",
            checker.diagnostics
        );
        assert_eq!(result, TypeStore::STRING);
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
            object: Box::new(ast::Expression::Identifier(ident("my_tuple"))),
            prop: Some(ast::MemberProp::Index(ast::IntLiteral {
                value: 1,
                loc: Location::dummy(),
            })),
            loc: Location::dummy(),
        };

        let result = checker.visit_tuple_indexing(&tuple_indexing);
        assert_eq!(result, TypeStore::STRING);
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
            object: Box::new(ast::Expression::Identifier(ident("not_a_tuple"))),
            prop: Some(ast::MemberProp::Index(ast::IntLiteral {
                value: 0,
                loc: Location::dummy(),
            })),
            loc: Location::dummy(),
        };

        let result = checker.visit_tuple_indexing(&tuple_indexing);
        assert_eq!(result, TypeStore::UNKNOWN);
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
            object: Box::new(ast::Expression::Identifier(ident("my_tuple"))),
            prop: Some(ast::MemberProp::Index(ast::IntLiteral {
                value: 2,
                loc: Location::dummy(),
            })),
            loc: Location::dummy(),
        };

        let result = checker.visit_tuple_indexing(&tuple_indexing);
        assert_eq!(result, TypeStore::UNKNOWN);
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
            object: Box::new(ast::Expression::Identifier(ident("my_tuple"))),
            prop: Some(ast::MemberProp::Index(ast::IntLiteral {
                value: -1,
                loc: Location::dummy(),
            })),
            loc: Location::dummy(),
        };

        let result = checker.visit_tuple_indexing(&tuple_indexing);
        assert_eq!(result, TypeStore::UNKNOWN);
        assert_eq!(checker.diagnostics.len(), 1);
    }
}
