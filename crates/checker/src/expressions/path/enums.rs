use tine_ast::PathSegment;
use tine_common::diagnostics::DiagnosticKind;
use tine_ir as ir;
use tine_symbols::symbols::EnumSymbolId;

use crate::expressions::path::{
    utils::get_host_type, visit_static_method_path, PathContext, PathVisitor,
};

pub(super) fn visit_enum_path(
    visitor: &mut PathVisitor,
    sym: EnumSymbolId,
    ctx: PathContext,
) -> Option<ir::Expression> {
    match ctx {
        PathContext::Expr => visit_enum_path_in_expr(visitor, sym),
        PathContext::Call => visit_enum_path_in_call(visitor, sym),
        PathContext::Struct => visit_enum_path_in_struct(visitor),
    }
}

fn visit_enum_path_in_expr(visitor: &mut PathVisitor, sym: EnumSymbolId) -> Option<ir::Expression> {
    let Some(variant_segment) = visitor.segments.pop_front() else {
        visitor
            .tc
            .error(DiagnosticKind::ExpectedVariantUnit, visitor.start_loc);
        return None;
    };
    match find_enum_variant(visitor, sym, &variant_segment) {
        Ok(v) => v,
        Err(()) => {
            let symbol = visitor.tc.symbols.get(sym);
            let enum_name = symbol.name.clone();
            let variant = variant_segment.ident.text;
            let error = DiagnosticKind::UnknownVariant { enum_name, variant };
            visitor.tc.error(error, variant_segment.loc);
            return None;
        }
    }
}

fn visit_enum_path_in_call(visitor: &mut PathVisitor, sym: EnumSymbolId) -> Option<ir::Expression> {
    let Some(next_segment) = visitor.segments.pop_front() else {
        visitor
            .tc
            .error(DiagnosticKind::ExpectedMember, visitor.start_loc);
        return None;
    };

    if let Ok(expr) = find_enum_variant(visitor, sym, &next_segment) {
        return expr;
    }

    visit_static_method_path(visitor, sym.into(), &next_segment)
}

fn find_enum_variant(
    visitor: &mut PathVisitor,
    sym: EnumSymbolId,
    segment: &PathSegment,
) -> Result<Option<ir::Expression>, ()> {
    let symbol = visitor.tc.symbols.get(sym);
    let variant = symbol
        .variants
        .iter()
        .find(|v| visitor.tc.symbol_name(**v) == segment.ident.as_str());
    let Some(&variant) = variant else {
        return Err(());
    };
    if segment.generic_args.is_some() {
        visitor
            .tc
            .error(DiagnosticKind::UnexpectedTypeParams, segment.loc);
    }
    if let Some(extra) = visitor.segments.pop_front() {
        let member = extra.ident.text;
        visitor.error(DiagnosticKind::UnknownMember { member }, extra.loc);
        return Ok(None);
    }
    let variant_symbol = visitor.tc.symbols.get(variant);
    if variant_symbol.body.len() > 0 {
        visitor.error(DiagnosticKind::ExpectedValueGotType, visitor.start_loc);
        return Ok(None);
    }
    let ty = visitor.tc.symbol_type_id(sym);
    let ty = get_host_type(visitor, ty);
    Ok(Some(ir::Expression::Identifier(ir::Identifier {
        ty,
        loc: segment.loc,
        symbol: variant.into(),
    })))
}

fn visit_enum_path_in_struct(visitor: &mut PathVisitor) -> Option<ir::Expression> {
    visitor
        .tc
        .error(DiagnosticKind::ExpectedStructGotEnum, visitor.start_loc);
    None
}
