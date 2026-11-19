mod enums;
mod structs;
mod utils;
mod variable;

use std::collections::VecDeque;

use tine_ast as ast;
use tine_common::diagnostics::DiagnosticKind;
use tine_ir as ir;
use tine_symbols::symbols::*;

use crate::TypeChecker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathContext {
    Expr,
    Call,
    Struct,
}

impl TypeChecker {
    fn get_path_first_symbol(&mut self, path: &ast::PathExpression) -> Option<SymbolId> {
        let name = path.segments[0].ident.as_str();
        let symbol = self.get_symbol_id(name);
        if symbol.is_none() {
            let error = DiagnosticKind::CannotFindName {
                name: name.to_string(),
            };
            self.error(error, path.segments[0].ident.loc);
        }
        symbol
    }

    pub fn visit_path_expression(
        &mut self,
        expr: ast::PathExpression,
        ctx: PathContext,
    ) -> Option<ir::Expression> {
        let first_symbol = self.get_path_first_symbol(&expr)?;
        let segments = VecDeque::from(expr.segments);
        use SymbolId::*;
        match first_symbol {
            Variable(_) => self.visit_variable_path(segments),
            Function(s) => self.visit_function_path(s, segments),
            Struct(s) => self.visit_struct_path(s, segments, ctx),
            Primitive(s) => self.visit_primitive_path(s, segments, ctx),
            Enum(s) => self.visit_enum_path(s, segments, ctx),
            Variant(s) => self.visit_variant_path(s, segments, ctx),

            TypeAlias(s) => self.visit_type_alias_path(s, segments, ctx),

            Trait(_) => self.visit_trait_path(segments),
            Member(_) | Method(_) => {
                panic!("shouldn't be able to find those symbols here!")
            }
        }
    }

    fn visit_function_path(
        &mut self,
        sym: FunctionSymbolId,
        mut segments: VecDeque<ast::PathSegment>,
    ) -> Option<ir::Expression> {
        let segment = segments.pop_front().unwrap();
        self.assert_path_end(&segments).ok()?;
        Some(
            self.visit_first_segment_with_args(segment, sym.into())
                .into(),
        )
    }

    fn visit_primitive_path(
        &mut self,
        sym: PrimitiveTypeSymbolId,
        mut segments: VecDeque<ast::PathSegment>,
        ctx: PathContext,
    ) -> Option<ir::Expression> {
        let segment = segments.pop_front().unwrap();
        if ctx != PathContext::Call {
            self.error(DiagnosticKind::UnexpectedType, segment.loc);
            return None;
        }
        if segments.is_empty() {
            self.error(DiagnosticKind::ExpectedFunctionGotType, segment.loc);
            return None;
        };
        self.visit_static_method_path(sym.into(), segments)
    }

    fn visit_trait_path(
        &mut self,
        mut segments: VecDeque<ast::PathSegment>,
    ) -> Option<ir::Expression> {
        let segment = segments.pop_front().unwrap();
        let error = DiagnosticKind::TraitNotAllowedInExpression;
        self.error(error, segment.loc);
        None
    }

    fn visit_type_alias_path(
        &mut self,
        sym: TypeAliasSymbolId,
        segments: VecDeque<ast::PathSegment>,
        ctx: PathContext,
    ) -> Option<ir::Expression> {
        let symbol = self.symbols.get(sym);
        match self.resolve_type_symbol(symbol.ty) {
            Some(TypeSymbolId::Struct(s)) => self.visit_struct_path(s, segments, ctx),
            Some(TypeSymbolId::Enum(s)) => self.visit_enum_path(s, segments, ctx),
            Some(TypeSymbolId::Primitive(s)) => self.visit_primitive_path(s, segments, ctx),
            Some(TypeSymbolId::Trait(_)) => self.visit_trait_path(segments),
            None => {
                let loc = segments[0].loc;
                self.error(DiagnosticKind::InvalidMember, loc);
                None
            }
        }
    }

    fn visit_variant_path(
        &mut self,
        sym: VariantSymbolId,
        mut segments: VecDeque<ast::PathSegment>,
        ctx: PathContext,
    ) -> Option<ir::Expression> {
        let segment = segments.pop_front().unwrap();
        let loc = segment.loc;
        if ctx == PathContext::Struct {
            self.error(DiagnosticKind::ExpectedStructGotEnum, loc);
            return None;
        }
        self.assert_path_end(&segments).ok()?;
        self.symbols.get_mut(sym).access().read(loc);
        Some(ir::Expression::Identifier(ir::Identifier {
            loc: loc,
            ty: self.symbol_type_id(sym),
            symbol: sym.into(),
        }))
    }
}
