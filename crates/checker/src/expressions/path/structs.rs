use std::collections::VecDeque;

use tine_ast as ast;
use tine_common::diagnostics::DiagnosticKind;
use tine_ir as ir;
use tine_symbols::symbols::*;

use crate::{expressions::path::PathContext, TypeChecker};

impl TypeChecker {
    pub(super) fn visit_struct_path(
        &mut self,
        sym: StructSymbolId,
        mut segments: VecDeque<ast::PathSegment>,
        ctx: PathContext,
    ) -> Option<ir::Expression> {
        let first = segments.pop_front().unwrap();
        let struct_ = self.visit_first_segment_with_args(first, sym.into());
        match ctx {
            PathContext::Expr => self.visit_struct_in_any_expr(sym, struct_, segments),
            PathContext::Call => self.visit_struct_path_in_call(sym, struct_, segments),
            PathContext::Struct => self.visit_struct_in_struct_expr(struct_, segments),
        }
    }

    /// Visit structs used as identifiers.
    ///
    /// This is forbidden by the language's grammar, except for unit structs,
    /// which can be constructed with `StructName` instead of `StructName {}`.
    fn visit_struct_in_any_expr(
        &mut self,
        sym: StructSymbolId,
        ident: ir::Identifier,
        segments: VecDeque<ast::PathSegment>,
    ) -> Option<ir::Expression> {
        self.assert_path_end(&segments).ok()?;
        let symbol = self.symbols.get(sym);
        if symbol.members.len() > 0 {
            self.error(DiagnosticKind::ExpectedValueGotType, ident.loc);
            return None;
        }
        Some(ir::Expression::Struct(ir::StructExpression {
            ty: ident.ty,
            loc: ident.loc,
            constructor: (ident.loc, sym),
            fields: vec![],
        }))
    }

    fn visit_struct_path_in_call(
        &mut self,
        sym: StructSymbolId,
        ident: ir::Identifier,
        segments: VecDeque<ast::PathSegment>,
    ) -> Option<ir::Expression> {
        // expect a method name after the host
        if segments.is_empty() {
            self.error(DiagnosticKind::ExpectedFunctionGotType, ident.loc);
            return None;
        };
        self.visit_static_method_path(sym.into(), segments)
    }

    /// Visit structs used in a constructor context (ie followed by braces).
    fn visit_struct_in_struct_expr(
        &mut self,
        ident: ir::Identifier,
        segments: VecDeque<ast::PathSegment>,
    ) -> Option<ir::Expression> {
        self.assert_path_end(&segments).ok()?;
        Some(ir::Expression::Identifier(ident))
    }
}
