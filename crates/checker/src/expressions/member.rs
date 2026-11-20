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
        use ast::MemberProp::*;
        let object = expr.object.map(|e| *e);
        match (object, expr.prop) {
            (object, Some(FieldName(field))) => self.visit_field_access(object, field, expr.loc),
            (object, Some(Index(index))) => self.visit_tuple_indexing(object, index, expr.loc),
            (None, None) => match self.this_type() {
                Some(ty) => Some(ir::Expression::This(ir::ThisExpression {
                    loc: expr.loc,
                    ty,
                })),
                None => {
                    self.error(DiagnosticKind::MissingExpression, expr.loc);
                    None
                }
            },
            // missing member already reported during parsing phase
            (Some(o), None) => {
                self.visit_expression(o);
                None
            }
        }
    }

    fn visit_field_access(
        &mut self,
        object: Option<ast::Expression>,
        field: ast::Identifier,
        loc: Location,
    ) -> Option<ir::Expression> {
        let object = match object {
            Some(o) => Some(self.visit_expression(o)?),
            None => None,
        };
        let Some(object_ty) = self.get_object_ty(&object) else {
            self.error(DiagnosticKind::MissingExpression, loc.nth_char(0));
            return None;
        };

        if let Some((symbol, ty)) = self.find_field(object_ty, field.as_str()) {
            return Some(self.visit_member_access(object, symbol, field, ty).into());
        }
        if let Some((symbol, ty)) = self.find_method(object_ty, field.as_str(), |m| !m.is_static())
        {
            return Some(self.visit_method_access(object, symbol, field, ty).into());
        }

        let member = field.as_str().to_string();
        let error = DiagnosticKind::UnknownMember { member };
        self.error(error, field.loc);
        None
    }

    fn visit_member_access(
        &mut self,
        object: Option<ir::Expression>,
        symbol: MemberSymbolId,
        field: ast::Identifier,
        ty: types::TypeId,
    ) -> ir::MemberExpression {
        self.read_symbol(symbol, field.loc);
        if !self.is_visible((symbol).into()) {
            let name = field.text.clone();
            let error = DiagnosticKind::FieldIsPrivate(name);
            self.error(error, field.loc);
        }
        let loc = match &object {
            Some(o) => Location::merge(o.loc(), field.loc),
            None => field.loc,
        };
        ir::MemberExpression {
            loc,
            object: object.map(Box::new),
            ty,
            member: (field.loc, symbol),
        }
    }

    fn visit_method_access(
        &mut self,
        object: Option<ir::Expression>,
        symbol: MethodSymbolId,
        field: ast::Identifier,
        ty: types::TypeId,
    ) -> ir::MethodExpression {
        let object_mutability = match &object {
            Some(o) => self.is_mutable(o),
            None => self.mutable_this,
        };
        let is_method_mutating = self.symbols.get(symbol).is_mutating();
        if is_method_mutating && object_mutability == Some(false) {
            self.error(DiagnosticKind::MutatingMethodOnImmutable, field.loc);
        }
        if !self.is_visible(symbol.into()) {
            let error = DiagnosticKind::MethodIsPrivate(field.text.clone());
            self.error(error, field.loc);
        }

        // This error will be removed later if actually called
        self.error(DiagnosticKind::NonCalledMethod, field.loc);

        let loc = match &object {
            Some(o) => Location::merge(o.loc(), field.loc),
            None => field.loc,
        };

        ir::MethodExpression {
            loc,
            host: object.map(Box::new),
            ty,
            method: (field.loc, symbol),
            args: vec![],
        }
    }

    pub fn visit_tuple_indexing(
        &mut self,
        object: Option<ast::Expression>,
        index: ast::IntLiteral,
        loc: Location,
    ) -> Option<ir::Expression> {
        let object = match object {
            Some(o) => Some(self.visit_expression(o)?),
            None => None,
        };
        let Some(object_ty) = self.get_object_ty(&object) else {
            self.error(DiagnosticKind::MissingExpression, loc.nth_char(0));
            return None;
        };
        let types::Type::Tuple(ty) = self.resolve(object_ty) else {
            if object_ty != TypeStore::UNKNOWN {
                let got = self.types.display(object_ty);
                let error = DiagnosticKind::ExpectedTuple { got };
                self.error(error, loc);
            }
            return None;
        };
        let elements = &ty.elements;

        match validate_index_in_range(index.value, elements.len()) {
            Ok(i) => Some(ir::Expression::Index(ir::IndexExpression {
                loc,
                object: object.map(Into::into),
                ty: ty.elements[i],
                index: (index.loc, i),
            })),
            Err(diag) => {
                self.error(diag, index.loc);
                None
            }
        }
    }

    pub fn get_struct_symbol(&self, mut ty: types::TypeId) -> Option<&StructSymbol> {
        while let types::Type::Ref(r) = self.resolve(ty) {
            ty = r.inner
        }
        self.symbols.find::<StructSymbolId, _>(|s| s.ty == ty)
    }

    fn get_object_ty(&self, object: &Option<ir::Expression>) -> Option<types::TypeId> {
        use types::Type::*;
        match object {
            Some(o) => match self.resolve(o.ty()) {
                Signal(t) => Some(t.inner),
                Listener(t) => Some(t.inner),
                _ => Some(o.ty()),
            },
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
            .or_else(|| {
                self.symbols
                    .find_id::<TraitSymbolId, _>(|s| s.ty == ty)
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

fn validate_index_in_range(index: i64, tuple_size: usize) -> Result<usize, DiagnosticKind> {
    if index < 0 {
        return Err(DiagnosticKind::NegativeTupleIndex);
    }
    let index = index as usize;
    if index >= tuple_size {
        let member = index.to_string();
        return Err(DiagnosticKind::UnknownMember { member });
    }
    Ok(index)
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
