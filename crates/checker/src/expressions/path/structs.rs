use tine_common::diagnostics::DiagnosticKind;
use tine_ir as ir;
use tine_symbols::symbols::*;

use crate::{
    assert_end,
    expressions::path::{
        utils::{get_host_type, visit_static_method_path},
        PathContext, PathVisitor,
    },
};

pub(super) fn visit_struct_path(
    visitor: &mut PathVisitor,
    sym: StructSymbolId,
    ctx: PathContext,
) -> Option<ir::Expression> {
    match ctx {
        PathContext::Expr => visit_struct_in_any_expr(visitor, sym),
        PathContext::Call => visit_struct_path_in_call(visitor, sym),
        PathContext::Struct => visit_struct_in_struct_expr(visitor, sym),
    }
}

fn visit_struct_in_any_expr(
    visitor: &mut PathVisitor,
    sym: StructSymbolId,
) -> Option<ir::Expression> {
    let symbol = visitor.tc.symbols.get(sym);
    if symbol.members.len() > 0 {
        visitor.error(DiagnosticKind::ExpectedValueGotType, visitor.start_loc);
        return None;
    }
    assert_end!(visitor);
    let ty = visitor.tc.symbol_type_id(sym);
    let ty = get_host_type(visitor, ty);
    Some(ir::Expression::Struct(ir::StructExpression {
        ty,
        loc: visitor.start_loc,
        constructor: (visitor.start_loc, sym),
        fields: vec![],
    }))
}

fn visit_struct_path_in_call(
    visitor: &mut PathVisitor,
    sym: StructSymbolId,
) -> Option<ir::Expression> {
    let Some(segment) = visitor.segments.pop_front() else {
        visitor.error(DiagnosticKind::ExpectedFunctionGotType, visitor.start_loc);
        return None;
    };
    visit_static_method_path(visitor, sym.into(), &segment)
}

fn visit_struct_in_struct_expr(
    visitor: &mut PathVisitor,
    sym: StructSymbolId,
) -> Option<ir::Expression> {
    if let Some(extra) = visitor.segments.pop_front() {
        let member = extra.ident.text;
        let error = DiagnosticKind::UnknownMember { member };
        visitor.tc.error(error, extra.loc);
        return None;
    }
    let host = visitor.tc.symbol_type_id(sym);
    let host = get_host_type(visitor, host);
    Some(ir::Expression::Identifier(ir::Identifier {
        loc: visitor.start_loc,
        ty: host,
        symbol: sym.into(),
    }))
}
