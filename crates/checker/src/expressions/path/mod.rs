mod enums;
mod structs;
mod utils;
mod variable;

use std::collections::VecDeque;

use tine_ast as ast;
use tine_common::{diagnostics::DiagnosticKind, locations::Location};
use tine_ir as ir;
use tine_symbols::symbols::*;
use tine_types::types;

use crate::{
    assert_end,
    expressions::path::{
        enums::visit_enum_path, structs::visit_struct_path, utils::visit_static_method_path,
        variable::visit_variable_path,
    },
    substitutions::Substitutions,
    TypeChecker,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathContext {
    Expr,
    Call,
    Struct,
}

pub(super) struct PathVisitor<'tc> {
    pub tc: &'tc mut TypeChecker,
    pub segments: VecDeque<ast::PathSegment>,
    pub first_generic_args: Option<Vec<types::TypeId>>,
    pub start_loc: Location,
}
impl<'a> PathVisitor<'a> {
    fn init(
        tc: &'a mut TypeChecker,
        mut segments: VecDeque<ast::PathSegment>,
    ) -> Option<(Self, SymbolId)> {
        let first = segments.pop_front()?;
        let first_generic_args = first
            .generic_args
            .map(|a| a.into_iter().map(|a| tc.visit_type(a)).collect::<Vec<_>>());
        let Some(symbol_id) = tc.get_symbol_id(first.ident.as_str()) else {
            let name = first.ident.text;
            let error = DiagnosticKind::CannotFindName { name };
            tc.error(error, first.ident.loc);
            return None;
        };

        let start_loc = first.ident.loc;
        tc.symbols
            .get_symbol_mut(symbol_id)
            .access()
            .read(first.ident.loc);
        Some((
            Self {
                tc,
                segments,
                first_generic_args,
                start_loc,
            },
            symbol_id,
        ))
    }

    fn error(&mut self, kind: DiagnosticKind, loc: Location) {
        self.tc.error(kind, loc)
    }
}

impl TypeChecker {
    pub fn visit_path_expression(
        &mut self,
        expr: ast::PathExpression,
        ctx: PathContext,
    ) -> Option<ir::Expression> {
        let segments = VecDeque::from(expr.segments);
        let (mut visitor, symbol_id) = PathVisitor::init(self, segments)?;
        use SymbolId::*;
        match symbol_id {
            Variable(s) => visit_variable_path(&mut visitor, s),
            Function(s) => visit_function_path(&mut visitor, s),
            Struct(s) => visit_struct_path(&mut visitor, s, ctx),
            Primitive(s) => visit_primitive_path(&mut visitor, s, ctx),
            Enum(s) => visit_enum_path(&mut visitor, s, ctx),

            TypeAlias(s) => visit_type_alias_path(&mut visitor, s, ctx),

            Trait(_) => {
                let error = DiagnosticKind::TraitNotAllowedInExpression;
                visitor.error(error, visitor.start_loc);
                None
            }
            Variant(_) | Member(_) | Method(_) => {
                panic!("shouldn't be able to find those symbols here!")
            }
        }
    }
}

fn visit_function_path(visitor: &mut PathVisitor, sym: FunctionSymbolId) -> Option<ir::Expression> {
    assert_end!(visitor);
    let ty = visitor.tc.symbol_type_id(sym);
    let f = visitor.tc.resolve(ty);
    let f = f.as_function().unwrap();
    let type_args = visitor.first_generic_args.clone().unwrap_or(vec![]);
    let sub = Substitutions::with_initial(&f.type_params, &type_args);
    let ty = sub.apply(&mut visitor.tc.types, ty);
    Some(ir::Expression::Identifier(ir::Identifier {
        loc: visitor.start_loc,
        ty,
        symbol: sym.into(),
    }))
}

fn visit_primitive_path(
    visitor: &mut PathVisitor,
    sym: PrimitiveTypeSymbolId,
    ctx: PathContext,
) -> Option<ir::Expression> {
    if ctx != PathContext::Call {
        visitor
            .tc
            .error(DiagnosticKind::UnexpectedType, visitor.start_loc);
        return None;
    }
    let Some(segment) = visitor.segments.pop_front() else {
        visitor.error(DiagnosticKind::ExpectedFunctionGotType, visitor.start_loc);
        return None;
    };
    visit_static_method_path(visitor, sym.into(), &segment)
}

fn visit_type_alias_path(
    visitor: &mut PathVisitor,
    sym: TypeAliasSymbolId,
    ctx: PathContext,
) -> Option<ir::Expression> {
    let symbol = visitor.tc.symbols.get(sym);
    match visitor.tc.resolve_type_symbol(symbol.ty) {
        Some(TypeSymbolId::Struct(s)) => visit_struct_path(visitor, s, ctx),
        Some(TypeSymbolId::Enum(s)) => visit_enum_path(visitor, s, ctx),
        Some(TypeSymbolId::Primitive(s)) => visit_primitive_path(visitor, s, ctx),
        None => visit_raw_type_path(visitor, sym, ctx),
    }
}

fn visit_raw_type_path(
    visitor: &mut PathVisitor,
    _sym: TypeAliasSymbolId,
    _ctx: PathContext,
) -> Option<ir::Expression> {
    // TODO
    visitor.error(DiagnosticKind::InvalidMember, visitor.start_loc);
    None
}
