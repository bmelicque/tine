use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};
use tine_ir::{self as ir, Typed};
use tine_symbols::symbols::*;
use tine_types::types;

use crate::{expressions::path::PathVisitor, substitutions::Substitutions, TypeChecker};

pub(super) fn visit_variable_path(
    visitor: &mut PathVisitor,
    sym: VariableSymbolId,
) -> Option<ir::Expression> {
    if visitor.first_generic_args.is_some() {
        visitor.error(DiagnosticKind::UnexpectedTypeParams, visitor.start_loc);
    }

    let ty = visitor.tc.symbol_type_id(sym);
    let mut base = ir::Expression::Identifier(ir::Identifier {
        loc: visitor.start_loc,
        ty,
        symbol: sym.into(),
    });

    while let Some(segment) = visitor.segments.pop_front() {
        base = visit_chain_segment(visitor, base, segment)?;
    }

    Some(base)
}

fn visit_chain_segment(
    visitor: &mut PathVisitor,
    host: ir::Expression,
    segment: ast::PathSegment,
) -> Option<ir::Expression> {
    let (host_ty, generic_args) = match visitor.tc.resolve(host.ty()) {
        types::Type::Ref(r) => (r.inner, r.args),
        _ => (host.ty(), vec![]),
    };
    let member_name = segment.ident.as_str();
    let variable_prop = find_variable_prop(&mut visitor.tc, host_ty, &generic_args, member_name);
    if let Some((sym, ty)) = variable_prop {
        return Some(ir::Expression::Member(ir::MemberExpression {
            loc: Location::merge(host.loc(), segment.loc),
            object: Some(Box::new(host)),
            member: (segment.loc, sym),
            ty,
        }));
    }

    if let Some((sym, ty)) = visitor.tc.find_method(host.ty(), member_name, |_| true) {
        return Some(ir::Expression::Method(ir::MethodExpression {
            loc: Location::merge(host.loc(), segment.loc),
            host: Some(Box::new(host)),
            method: (segment.loc, sym),
            args: vec![],
            ty,
        }));
    }

    None
}

fn find_variable_prop(
    tc: &mut TypeChecker,
    host: types::TypeId,
    generic_args: &[types::TypeId],
    prop: &str,
) -> Option<(MemberSymbolId, types::TypeId)> {
    let symbol = tc.get_struct_symbol(host)?;
    let member = symbol
        .members
        .iter()
        .find(|m| tc.symbol_name(**m) == prop)
        .copied()?;
    let struct_ty = tc.resolve(symbol.ty);
    let struct_ty = struct_ty.as_struct().unwrap();
    let sub = Substitutions::with_initial(&struct_ty.params, generic_args);
    let member_ty = tc.symbol_type_id(member);
    let member_ty = sub.apply(&mut tc.types, member_ty);
    Some((member, member_ty))
}
