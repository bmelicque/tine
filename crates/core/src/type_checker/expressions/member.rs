use crate::{
    ast, ir,
    type_checker::{
        analysis_context::{symbols::TypeSymbolBody, type_store::TypeStore},
        substitutions::{SubstitutionTable, Substitutions},
        TypeChecker,
    },
    types, DiagnosticKind, SymbolKind, SymbolRef,
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
        let substitutions = self.infer_type_args(&root_symbol, object.ty());

        let SymbolKind::Struct {
            body: TypeSymbolBody::Struct(fields),
            methods,
        } = &root_symbol.borrow().kind
        else {
            panic!();
        };

        let field = fields.iter().find(|(name, _)| *name == field_name.text);
        if let Some((_, symbol)) = field {
            let ty = substitutions.apply(&mut self.session.types(), symbol.as_type());
            return Some(ir::MemberExpression {
                loc: expr.loc,
                object: Box::new(object),
                ty,
                member: ir::Identifier {
                    loc: field_name.loc,
                    symbol: symbol.to_owned(),
                },
            });
        }

        let matching_methods = methods
            .iter()
            .filter(|m| {
                self.method_matches(m, field_name.as_str(), object.is_mutable(), &substitutions)
            })
            .collect::<Vec<_>>();

        if matching_methods.len() == 0 {
            let error = DiagnosticKind::UnknownMember {
                member: field_name.as_str().to_string(),
            };
            self.error(error, field_name.loc);
            return None;
        }

        let most_concrete = matching_methods
            .into_iter()
            .max_by_key(|m| method_concreteness(m))?;
        let object_mutablity = object.is_mutable();
        let is_method_mutating = match &most_concrete.borrow().kind {
            SymbolKind::Method { receiver, .. } => receiver.is_mutable(), // TODO: handle mutability
            // Other symbol kinds should have been filtered out above
            _ => unreachable!(),
        };
        if is_method_mutating && object_mutablity == Some(false) {
            self.error(DiagnosticKind::MutatingMethodOnImmutable, field_name.loc);
        }

        let ty = substitutions.apply(&mut self.session.types(), most_concrete.as_type());

        Some(ir::MemberExpression {
            loc: expr.loc,
            object: Box::new(object),
            ty,
            member: ir::Identifier {
                loc: field_name.loc,
                symbol: most_concrete.to_owned(),
            },
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

        let types::Type::Tuple(ty) = self.resolve(object.ty()) else {
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

    fn method_matches(
        &self,
        symbol: &SymbolRef,
        name: &str,
        mutable: Option<bool>,
        type_args: &Substitutions,
    ) -> bool {
        // Keeping methods with same name
        if symbol.borrow().name != name {
            return false;
        }
        let SymbolKind::Method {
            owner_args,
            receiver,
            ..
        } = &symbol.borrow().kind
        else {
            return false;
        };

        // TODO: remove this
        if receiver.is_mutable() && mutable == Some(false) {
            return false;
        }

        // No type args or generic implementation
        if owner_args.len() == 0 {
            return true;
        }

        return *owner_args == SubstitutionTable::from(type_args);
    }
}

fn method_concreteness(method: &SymbolRef) -> usize {
    let SymbolKind::Method { owner_args, .. } = &method.borrow().kind else {
        panic!()
    };
    owner_args.len()
}
