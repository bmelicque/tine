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
