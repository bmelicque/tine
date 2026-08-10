use tine_ast::PathSegment;
use tine_common::diagnostics::DiagnosticKind;
use tine_ir as ir;
use tine_symbols::symbols::{MethodReceiverKind, SymbolId};
use tine_types::types;

use crate::{expressions::path::PathVisitor, substitutions::Substitutions};

pub(super) fn visit_static_method_path(
    visitor: &mut PathVisitor,
    sym: SymbolId,
    segment: &PathSegment,
) -> Option<ir::Expression> {
    assert_end!(visitor);
    let host = visitor.tc.symbol_type_id(sym);
    let host = get_host_type(visitor, host);
    let symbol = visitor.tc.find_method(host, segment.ident.as_str(), |m| {
        m.receiver == MethodReceiverKind::Static
    });
    let Some((symbol, ty)) = symbol else {
        let member = segment.ident.text.clone();
        visitor.error(DiagnosticKind::UnknownMember { member }, segment.loc);
        return None;
    };
    Some(ir::Expression::Identifier(ir::Identifier {
        ty,
        loc: segment.loc,
        symbol: symbol.into(),
    }))
}

pub fn get_host_type(visitor: &mut PathVisitor, ty: types::TypeId) -> types::TypeId {
    match &visitor.first_generic_args {
        Some(args) => {
            let resolved = visitor.tc.resolve(ty);
            let params = &resolved.as_struct().unwrap().params;
            let sub = Substitutions::with_initial(params, &args);
            sub.apply(&mut visitor.tc.types, ty)
        }
        None => ty,
    }
}

#[macro_export]
macro_rules! assert_end {
    ($visitor:expr) => {
        if let Some(extra) = $visitor.segments.pop_front() {
            let member = extra.ident.text;
            $visitor.error(DiagnosticKind::UnknownMember { member }, extra.loc);
            return None;
        }
    };
}
use assert_end;
