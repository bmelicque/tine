use tine_ast as ast;
use tine_common::{diagnostics::DiagnosticKind, locations::Location};
use tine_ir::{self as ir, Typed};
use tine_symbols::symbols::*;
use tine_types::{store::TypeStore, types};

use crate::{
    substitutions::{SubstitutionTable, Substitutions},
    TypeChecker,
};

impl TypeChecker {
    pub fn visit_member_expression(
        &mut self,
        expr: ast::MemberExpression,
    ) -> Option<ir::Expression> {
        let Some(member) = &expr.prop else {
            expr.object.and_then(|o| self.visit_expression(*o));
            // missing member already reported during parsing phase
            return None;
        };
        match member {
            ast::MemberProp::FieldName(_) => self.visit_field_access(expr),
            ast::MemberProp::Index(_) => self.visit_tuple_indexing(expr).map(Into::into),
        }
    }

    fn visit_field_access(&mut self, expr: ast::MemberExpression) -> Option<ir::Expression> {
        debug_assert!(matches!(expr.prop, Some(ast::MemberProp::FieldName(_))));
        let object = match expr.object {
            Some(o) => Some(self.visit_expression(*o)?),
            None => None,
        };
        let Some(object_ty) = self.get_object_ty(&object) else {
            self.error(DiagnosticKind::MissingExpression, expr.loc.nth_char(0));
            return None;
        };
        let Some(ast::MemberProp::FieldName(field_name)) = expr.prop else {
            unreachable!()
        };
        let Some(root_symbol) = self.get_struct_symbol(object_ty).cloned() else {
            let error = DiagnosticKind::UnknownMember {
                member: field_name.as_str().to_string(),
            };
            self.error(error, field_name.loc);
            return None;
        };
        let substitutions = self.infer_type_args(&root_symbol, object_ty);

        let (object, field) = match self.visit_field_as_prop(
            &root_symbol,
            object,
            field_name,
            expr.loc,
            &substitutions,
        ) {
            Ok(expr) => return Some(expr.into()),
            Err(r) => r,
        };
        self.visit_field_as_method(
            object,
            field,
            &root_symbol.methods,
            expr.loc,
            &substitutions,
        )
        .map(Into::into)
    }

    /// Return the `root` back on error
    fn visit_field_as_prop(
        &mut self,
        root_symbol: &StructSymbol,
        root: Option<ir::Expression>,
        field: ast::Identifier,
        loc: Location,
        substitutions: &Substitutions,
    ) -> Result<ir::MemberExpression, (Option<ir::Expression>, ast::Identifier)> {
        let members = &root_symbol.members;

        let Some(symbol_id) = members
            .into_iter()
            .find(|m| self.symbol_name(**m) == field.as_str())
        else {
            return Err((root, field));
        };
        self.read_symbol(*symbol_id, field.loc);
        if !self.is_visible((*symbol_id).into()) {
            let error = DiagnosticKind::FieldIsPrivate(field.text.clone());
            self.error(error, field.loc);
        }
        let ty = self.symbol_type_id(*symbol_id);
        let ty = substitutions.apply(&mut self.types, ty);
        Ok(ir::MemberExpression {
            loc,
            object: root.map(Box::new),
            ty,
            member: (field.loc, *symbol_id),
        })
    }

    fn visit_field_as_method(
        &mut self,
        object: Option<ir::Expression>,
        field: ast::Identifier,
        methods: &Vec<MethodSymbolId>,
        loc: Location,
        substitutions: &Substitutions,
    ) -> Option<ir::MethodExpression> {
        let object_mutability = match &object {
            Some(o) => self.is_mutable(o),
            None => self.mutable_this,
        };
        let matching_methods = methods
            .into_iter()
            .filter(|m| self.method_matches(**m, field.as_str(), object_mutability, &substitutions))
            .collect::<Vec<_>>();

        if matching_methods.len() == 0 {
            let error = DiagnosticKind::UnknownMember {
                member: field.as_str().to_string(),
            };
            self.error(error, field.loc);
            return None;
        }

        let &most_concrete_id = matching_methods
            .into_iter()
            .max_by_key(|m| self.symbols.get(**m).concreteness())?;
        let is_method_mutating = self.symbols.get(most_concrete_id).is_mutating();
        if is_method_mutating && object_mutability == Some(false) {
            self.error(DiagnosticKind::MutatingMethodOnImmutable, field.loc);
        }
        if !self.is_visible(most_concrete_id.into()) {
            let error = DiagnosticKind::MethodIsPrivate(field.text.clone());
            self.error(error, field.loc);
        }

        let ty = self.symbol_type_id(most_concrete_id);
        let ty = substitutions.apply(&mut self.types, ty);

        self.error(DiagnosticKind::NonCalledMethod, field.loc);

        Some(ir::MethodExpression {
            loc: Location::merge(loc, field.loc),
            host: object.map(Box::new),
            ty,
            method: (field.loc, most_concrete_id),
            args: vec![],
        })
    }

    pub fn visit_tuple_indexing(
        &mut self,
        expr: ast::MemberExpression,
    ) -> Option<ir::IndexExpression> {
        let Some(ast::MemberProp::Index(index)) = &expr.prop else {
            panic!();
        };

        let object = match expr.object {
            Some(o) => Some(self.visit_expression(*o)?),
            None => None,
        };
        let Some(object_ty) = self.get_object_ty(&object) else {
            self.error(DiagnosticKind::MissingExpression, expr.loc.nth_char(0));
            return None;
        };
        let types::Type::Tuple(ty) = self.resolve(object_ty) else {
            if object_ty != TypeStore::UNKNOWN {
                let got = self.types.display(object_ty);
                let error = DiagnosticKind::ExpectedTuple { got };
                self.error(error, expr.loc);
            }
            return None;
        };
        let elements = &ty.elements;

        // check index is in range
        let value = index.value;
        if value < 0 {
            self.error(DiagnosticKind::NegativeTupleIndex, index.loc);
            return None;
        }
        let value = value as usize;
        if value >= elements.len() {
            let member = value.to_string();
            self.error(DiagnosticKind::UnknownMember { member }, index.loc);
            return None;
        }

        Some(ir::IndexExpression {
            loc: expr.loc,
            object: object.map(Into::into),
            ty: ty.elements[value],
            index: (index.loc, value),
        })
    }

    pub fn get_struct_symbol(&self, mut ty: types::TypeId) -> Option<&StructSymbol> {
        while let types::Type::Ref(r) = self.resolve(ty) {
            ty = r.inner
        }
        self.symbols.find::<StructSymbolId, _>(|s| s.ty == ty)
    }

    fn get_object_ty(&self, object: &Option<ir::Expression>) -> Option<types::TypeId> {
        match object {
            Some(o) => Some(o.ty()),
            None => self.this_type(),
        }
    }

    pub fn get_type_symbol_id(&self, mut ty: types::TypeId) -> Option<TypeSymbolId> {
        while let types::Type::Ref(r) = self.resolve(ty) {
            ty = r.inner
        }
        self.symbols
            .find_id::<StructSymbolId, _>(|s| s.ty == ty)
            .map(Into::into)
            .or_else(|| {
                self.symbols
                    .find_id::<EnumSymbolId, _>(|s| s.ty == ty)
                    .map(Into::into)
            })
            .or_else(|| {
                self.symbols
                    .find_id::<PrimitiveTypeSymbolId, _>(|s| s.ty == ty)
                    .map(Into::into)
            })
    }

    pub(crate) fn is_visible(&self, symbol: SymbolId) -> bool {
        if self.symbols.is_public(symbol) {
            return true;
        }
        self.symbols.get_symbol(symbol).defined_at().module() == self.current_module()
    }

    pub fn method_matches(
        &self,
        symbol_id: MethodSymbolId,
        name: &str,
        mutable: Option<bool>,
        type_args: &Substitutions,
    ) -> bool {
        let symbol = self.symbols.get(symbol_id);
        // Keeping methods with same name
        if symbol.name != name {
            return false;
        }

        if symbol.is_mutating() && mutable == Some(false) {
            return false;
        }

        symbol.concreteness() == 0
            || symbol.matches_substitutions(&SubstitutionTable::from(type_args))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visit_tuple_index() {
        let mut checker = TypeChecker::new();
        let ty = checker.intern(types::TupleType {
            params: vec![],
            elements: vec![TypeStore::INTEGER, TypeStore::STRING],
        });
        checker.insert::<VariableSymbolId>(VariableSymbol {
            name: "x".to_string(),
            ty,
            ..Default::default()
        });
        let node = ast::MemberExpression {
            loc: Location::dummy(),
            object: Some(Box::new(
                ast::Identifier::new("x".into(), Location::dummy()).into(),
            )),
            prop: Some(ast::IntLiteral::new(1, Location::dummy()).into()),
        };
        checker.visit_member_expression(node);
        assert!(checker.diagnostics.is_empty(), "{:#?}", checker.diagnostics);
    }
}
