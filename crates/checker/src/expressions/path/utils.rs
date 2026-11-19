use std::collections::VecDeque;

use tine_ast as ast;
use tine_common::diagnostics::DiagnosticKind;
use tine_ir as ir;
use tine_symbols::symbols::{MethodReceiverKind, MethodSymbol, SymbolId};

use crate::TypeChecker;

impl TypeChecker {
    pub(super) fn assert_path_end(
        &mut self,
        segments: &VecDeque<ast::PathSegment>,
    ) -> Result<(), ()> {
        match segments.len() {
            0 => Ok(()),
            _ => {
                let member = segments[0].ident.as_str().to_string();
                let loc = segments[0].loc;
                self.error(DiagnosticKind::UnknownMember { member }, loc);
                return Err(());
            }
        }
    }

    pub(super) fn visit_first_segment_with_args(
        &mut self,
        segment: ast::PathSegment,
        sym: SymbolId,
    ) -> ir::Identifier {
        let expressed_type_args = segment
            .generic_args
            .unwrap_or(vec![])
            .into_iter()
            .map(|a| self.visit_type(a))
            .collect::<Vec<_>>();
        let ty = self.symbol_type_id(sym);
        let type_args = self.types.complete_type_args(ty, expressed_type_args);
        let ty = self.types.instantiate(ty, &type_args);
        ir::Identifier {
            loc: segment.loc,
            ty,
            symbol: sym.into(),
        }
    }

    /// Visit a static method path, like `MyType.method.<Arg>`.
    ///
    /// The segment queue is expected to contain at least one element
    /// (the one containing the method's name).
    pub(super) fn visit_static_method_path(
        &mut self,
        host_symbol: SymbolId,
        mut segments: VecDeque<ast::PathSegment>,
    ) -> Option<ir::Expression> {
        let method_segment = segments.pop_front().unwrap();
        let method_name = method_segment.ident.as_str();
        let loc = method_segment.loc;
        self.assert_path_end(&segments).ok()?;
        let host = self.symbol_type_id(host_symbol);
        let symbol = self.find_method(host, method_name, is_static);
        let Some((symbol, ty)) = symbol else {
            let member = method_segment.ident.as_str().to_string();
            self.error(DiagnosticKind::UnknownMember { member }, loc);
            return None;
        };
        self.symbols.get_mut(symbol).access.read(loc);
        Some(ir::Expression::Identifier(ir::Identifier {
            ty,
            loc,
            symbol: symbol.into(),
        }))
    }
}

fn is_static(method: &MethodSymbol) -> bool {
    method.receiver == MethodReceiverKind::Static
}
