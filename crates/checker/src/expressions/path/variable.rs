use std::collections::VecDeque;

use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};
use tine_ir::{self as ir, Typed};
use tine_symbols::symbols::*;
use tine_types::types;

use crate::TypeChecker;

impl TypeChecker {
    /// Visit a path that starts with a variable name, like a member expression
    /// or a method call.
    pub(super) fn visit_variable_path(
        &mut self,
        mut segments: VecDeque<ast::PathSegment>,
    ) -> Option<ir::Expression> {
        let first = segments.pop_front().unwrap();
        if first.generic_args.is_some() {
            self.error(DiagnosticKind::UnexpectedTypeParams, first.loc);
        }
        let mut id = self.visit_identifier(first.ident)?;
        id.ty = self.types.instantiate(id.ty(), &[]);
        let mut expr = id.into();

        while let Some(segment) = segments.pop_front() {
            expr = self.visit_variable_path_segment(expr, segment)?;
        }

        Some(expr)
    }

    fn visit_variable_path_segment(
        &mut self,
        host: ir::Expression,
        segment: ast::PathSegment,
    ) -> Option<ir::Expression> {
        let ty: u32 = self.auto_deref(host.ty());
        let ident = segment.ident;
        let member_name = ident.as_str();

        // Try as field
        if let Some((sym, ty)) = self.find_field(ty, ident.as_str()) {
            self.symbols.get_mut(sym).access.read(ident.loc);
            if !self.is_visible(sym.into()) {
                let error = DiagnosticKind::FieldIsPrivate(member_name.to_string());
                self.error(error, ident.loc);
            }
            return Some(ir::Expression::Member(ir::MemberExpression {
                loc: Location::merge(host.loc(), segment.loc),
                object: Some(Box::new(host)),
                member: (segment.loc, sym),
                ty,
            }));
        }

        // Try as method
        if let Some((sym, ty)) = self.find_method(ty, member_name, |_| true) {
            self.symbols.get_mut(sym).access.read(ident.loc);
            if !self.is_visible(sym.into()) {
                let error = DiagnosticKind::MethodIsPrivate(member_name.to_string());
                self.error(error, ident.loc);
            }
            return Some(ir::Expression::Method(ir::MethodExpression {
                loc: Location::merge(host.loc(), segment.loc),
                host: Some(Box::new(host)),
                method: (segment.loc, sym),
                args: vec![],
                ty,
            }));
        }

        // Default to error
        let member = member_name.to_string();
        let error = DiagnosticKind::UnknownMember { member };
        self.error(error, segment.loc);

        None
    }

    /// Find the field with given name on the given type.
    ///
    /// This method expects the type to be instantiated.
    fn find_field(
        &self,
        host: types::TypeId,
        prop: &str,
    ) -> Option<(MemberSymbolId, types::TypeId)> {
        let symbol = self.get_struct_symbol(host)?;
        let member = symbol
            .members
            .iter()
            .find(|m| self.symbol_name(**m) == prop)
            .copied()?;
        let ty = self.resolve(symbol.ty);
        let struct_ty = ty.as_struct()?;
        let member_ty = struct_ty.fields.iter().find(|f| f.name == prop)?.def;
        Some((member, member_ty))
    }
}
