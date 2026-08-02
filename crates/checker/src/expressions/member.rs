use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};
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
        let object = expr.object.and_then(|o| self.visit_expression(*o))?;
        let Some(ast::MemberProp::FieldName(field_name)) = expr.prop else {
            unreachable!()
        };
        let Some(root_symbol) = self.get_struct_symbol(object.ty()).cloned() else {
            let error = DiagnosticKind::UnknownMember {
                member: field_name.as_str().to_string(),
            };
            self.error(error, field_name.loc);
            return None;
        };
        let substitutions = self.infer_type_args(&root_symbol, object.ty());

        let (object, field) =
            match self.visit_field_as_prop(&root_symbol, object, field_name, &substitutions) {
                Ok(expr) => return Some(expr.into()),
                Err(r) => r,
            };
        self.visit_field_as_method(object, field, &root_symbol.methods, &substitutions)
            .map(Into::into)
    }

    /// Return the `root` back on error
    fn visit_field_as_prop(
        &mut self,
        root_symbol: &StructSymbol,
        root: ir::Expression,
        field: ast::Identifier,
        substitutions: &Substitutions,
    ) -> Result<ir::MemberExpression, (ir::Expression, ast::Identifier)> {
        let fields = match &root_symbol.body {
            TypeSymbolBody::Struct(s) => s,
            _ => return Err((root, field)),
        };

        let Some((_, symbol_id)) = fields.iter().find(|(name, _)| *name == field.text) else {
            return Err((root, field));
        };
        if !self.is_visible((*symbol_id).into()) {
            let error = DiagnosticKind::FieldIsPrivate(field.text.clone());
            self.error(error, field.loc);
        }
        let ty = self.symbol_type_id(*symbol_id);
        let ty = substitutions.apply(&mut self.types, ty);
        Ok(ir::MemberExpression {
            loc: Location::merge(root.loc(), field.loc),
            object: Box::new(root),
            ty,
            member: (field.loc, *symbol_id),
        })
    }

    fn visit_field_as_method(
        &mut self,
        object: ir::Expression,
        field: ast::Identifier,
        methods: &Vec<MethodSymbolId>,
        substitutions: &Substitutions,
    ) -> Option<ir::MethodExpression> {
        let matching_methods = methods
            .into_iter()
            .filter(|m| {
                let is_mutable = self.is_mutable(&object);
                self.method_matches(**m, field.as_str(), is_mutable, &substitutions)
            })
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
        let object_mutablity = self.is_mutable(&object);
        let is_method_mutating = self.symbols.get(most_concrete_id).is_mutating();
        if is_method_mutating && object_mutablity == Some(false) {
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
            loc: Location::merge(object.loc(), field.loc),
            host: Box::new(object),
            ty,
            method: (field.loc, most_concrete_id),
            args: vec![],
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
        let Some(root_symbol) = self.get_struct_symbol(object.ty()).cloned() else {
            let error = DiagnosticKind::UnknownMember {
                member: index.value.to_string(),
            };
            self.error(error, index.loc);
            return None;
        };

        let types::Type::Tuple(ty) = self.resolve(object.ty()) else {
            if object.ty() != TypeStore::UNKNOWN {
                let error = DiagnosticKind::ExpectedTuple {
                    got: self.types.display(object.ty()),
                };
                self.error(error, object.loc());
            }
            return None;
        };
        let elements = match &root_symbol.body {
            TypeSymbolBody::Struct(s) => s,
            _ => panic!(),
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

        Some(ir::MemberExpression {
            loc: expr.loc,
            object: Box::new(object),
            ty: ty.elements[value],
            member: (index.loc, elements[value].1),
        })
    }

    fn get_struct_symbol(&self, mut ty: types::TypeId) -> Option<&StructSymbol> {
        while let types::Type::Ref(r) = self.resolve(ty) {
            ty = r.inner
        }
        self.symbols.find::<StructSymbolId, _>(|s| s.ty == ty)
    }

    pub(crate) fn is_visible(&self, symbol: SymbolId) -> bool {
        if self.symbols.is_public(symbol) {
            return true;
        }
        self.symbols.get_symbol(symbol).defined_at().module() == self.current_module()
    }

    fn method_matches(
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
