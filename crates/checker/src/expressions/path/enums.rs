use std::collections::VecDeque;

use tine_ast as ast;
use tine_common::{diagnostics::DiagnosticKind, locations::Location};
use tine_ir as ir;
use tine_symbols::symbols::EnumSymbolId;

use crate::{expressions::path::PathContext, TypeChecker};

impl TypeChecker {
    /// Visit a path starting with an enum name
    pub(super) fn visit_enum_path(
        &mut self,
        sym: EnumSymbolId,
        mut segments: VecDeque<ast::PathSegment>,
        ctx: PathContext,
    ) -> Option<ir::Expression> {
        let first = segments.pop_front().unwrap();
        let enum_ = self.visit_first_segment_with_args(first, sym.into());
        match ctx {
            PathContext::Expr => self.visit_enum_path_in_expr(enum_, segments),
            PathContext::Call => self.visit_enum_path_in_call(enum_, segments),
            PathContext::Struct => self.visit_enum_path_in_struct(enum_),
        }
    }

    /// Visit a path starting with an enum name outside of a call or construct
    /// context. This leaves only unit variant values (like `Option.None`) as
    /// valid expressions.
    fn visit_enum_path_in_expr(
        &mut self,
        enum_: ir::Identifier,
        mut segments: VecDeque<ast::PathSegment>,
    ) -> Option<ir::Expression> {
        let Some(variant_segment) = segments.pop_front() else {
            self.error(DiagnosticKind::ExpectedVariantUnit, enum_.loc);
            return None;
        };
        match find_enum_variant(self, &enum_, &variant_segment, segments) {
            Ok(Some((expr, 0))) => Some(expr),
            Ok(Some(_)) => {
                self.error(DiagnosticKind::ExpectedValueGotType, enum_.loc);
                None
            }
            Ok(None) => None,
            Err(()) => {
                let symbol = self.symbols.get(enum_.symbol.as_enum()?);
                let enum_name = symbol.name.clone();
                let variant = variant_segment.ident.text;
                let error = DiagnosticKind::UnknownVariant { enum_name, variant };
                self.error(error, variant_segment.loc);
                return None;
            }
        }
    }

    /// Visit a path starting with an enum identifier within a "call" context
    /// (ie parentheses), which could be either constructing a variant or
    /// calling a static method.
    fn visit_enum_path_in_call(
        &mut self,
        enum_: ir::Identifier,
        mut segments: VecDeque<ast::PathSegment>,
    ) -> Option<ir::Expression> {
        let Some(next_segment) = segments.pop_front() else {
            self.error(DiagnosticKind::ExpectedMember, enum_.loc);
            return None;
        };

        if let Ok(expr) = find_enum_variant(self, &enum_, &next_segment, segments.clone()) {
            if let Some((_, len)) = expr {
                if len == 0 {
                    self.error(DiagnosticKind::ExpectedTypeGotValue, enum_.loc);
                    return None;
                }
            }
            return expr.map(|e| e.0);
        }

        segments.push_front(next_segment);
        self.visit_static_method_path(enum_.symbol, segments)
    }

    fn visit_enum_path_in_struct(&mut self, enum_: ir::Identifier) -> Option<ir::Expression> {
        self.error(DiagnosticKind::ExpectedStructGotEnum, enum_.loc);
        None
    }
}

fn find_enum_variant(
    tc: &mut TypeChecker,
    enum_ident: &ir::Identifier,
    variant_segment: &ast::PathSegment,
    mut segments: VecDeque<ast::PathSegment>,
) -> Result<Option<(ir::Expression, usize)>, ()> {
    let sym = enum_ident.symbol.as_enum().unwrap();
    let symbol = tc.symbols.get(sym);
    let variant = symbol
        .variants
        .iter()
        .find(|v| tc.symbol_name(**v) == variant_segment.ident.as_str());
    let Some(&variant) = variant else {
        return Err(());
    };
    if variant_segment.generic_args.is_some() {
        tc.error(DiagnosticKind::UnexpectedTypeParams, variant_segment.loc);
    }
    if let Some(extra) = segments.pop_front() {
        let member = extra.ident.text;
        tc.error(DiagnosticKind::UnknownMember { member }, extra.loc);
        return Ok(None);
    }
    tc.symbols.get_mut(variant).access.read(variant_segment.loc);
    let variant_symbol = tc.symbols.get(variant).clone();

    let ty = enum_ident.ty;
    let id = ir::Expression::Identifier(ir::Identifier {
        ty,
        loc: Location::merge(enum_ident.loc, variant_segment.loc),
        symbol: variant.into(),
    });
    Ok(Some((id, variant_symbol.body.len())))
}
