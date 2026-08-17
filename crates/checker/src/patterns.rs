use std::collections::VecDeque;

use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};
use tine_ir::{self as ir, Typed};
use tine_symbols::symbols::*;
use tine_types::{store::display_type, types};

use crate::{
    expressions::expressions::{
        visit_boolean_literal, visit_float_literal, visit_int_literal, visit_string_literal,
    },
    substitutions::Substitutions,
    PathContext, TypeChecker,
};

pub(super) struct PatternVisitor<'deps, 'tc> {
    pub(super) is_public: bool,
    /// All the dependencies of the expression the pattern is matched against
    pub(super) dependencies: &'deps [ir::Identifier],
    pub(super) tc: &'tc mut TypeChecker,
}

fn visit_pattern(
    pattern: ast::Pattern,
    visitor: &mut PatternVisitor,
    expected: types::TypeId,
) -> Option<ir::Pattern> {
    use ast::Pattern::*;
    match pattern {
        Identifier(i) => visit_identifier_pattern(i, visitor, expected),
        Invalid(_) => None,
        Literal(l) => Some(visit_literal_pattern(l)),
        Path(p) => visit_path_pattern(p, visitor, expected),
        Call(p) => visit_call_pattern(p, visitor, expected).map(Into::into),
        Struct(c) => visit_struct_pattern(c, visitor, expected),
        Tuple(t) => visit_tuple_pattern(t, visitor, expected).map(Into::into),
    }
}

fn visit_identifier_pattern(
    pattern: ast::IdentifierPattern,
    visitor: &mut PatternVisitor,
    expected: types::TypeId,
) -> Option<ir::Pattern> {
    if let Some(call) = try_identifier_as_variant(&pattern, visitor, expected) {
        return Some(call);
    }
    let symbol = declare_variable(visitor, &pattern.identifier, expected, pattern.mutable)?;
    let identifier = ir::Identifier {
        loc: pattern.loc(),
        symbol: symbol.into(),
        ty: expected,
    };

    Some(identifier.into())
}
fn try_identifier_as_variant(
    pattern: &ast::IdentifierPattern,
    visitor: &mut PatternVisitor,
    expected: types::TypeId,
) -> Option<ir::Pattern> {
    let loc = pattern.loc();
    let type_symbol = visitor.tc.get_type_symbol_id(expected)?;
    let TypeSymbolId::Enum(e) = type_symbol else {
        return None;
    };
    let variant = *visitor
        .tc
        .symbols
        .get(e)
        .variants
        .iter()
        .find(|variant| visitor.tc.symbol_name(**variant) == pattern.identifier.as_str())?;
    Some(ir::Pattern::Call(ir::CallPattern {
        ty: expected,
        loc,
        callee: (loc, variant),
        arguments: vec![],
    }))
}

fn visit_path_pattern(
    pattern: ast::PathExpression,
    visitor: &mut PatternVisitor,
    expected: types::TypeId,
) -> Option<ir::Pattern> {
    let mut segments = VecDeque::from(pattern.segments);
    let first = segments.pop_front()?;
    if let Some(_) = first.generic_args {
        visitor.tc.error(DiagnosticKind::InvalidPattern, first.loc);
        return None;
    }
    let Some(symbol_id) = visitor.tc.get_symbol_id(first.ident.as_str()) else {
        let name = first.ident.text;
        let error = DiagnosticKind::CannotFindName { name };
        visitor.tc.error(error, first.ident.loc);
        return None;
    };
    visitor
        .tc
        .symbols
        .get_symbol_mut(symbol_id)
        .access()
        .read(first.ident.loc);
    let SymbolId::Enum(sym) = symbol_id else {
        visitor
            .tc
            .error(DiagnosticKind::ExpectedVariantUnit, pattern.loc);
        return None;
    };
    check_path_type(visitor, sym, expected, pattern.loc);

    let Some(variant_segment) = segments.pop_front() else {
        visitor
            .tc
            .error(DiagnosticKind::ExpectedVariantUnit, pattern.loc);
        return None;
    };
    if let Some(extra) = segments.pop_front() {
        let member = extra.ident.text;
        visitor
            .tc
            .error(DiagnosticKind::UnknownMember { member }, extra.loc);
        return None;
    }
    match find_enum_variant(visitor, sym, variant_segment) {
        Ok(v) => v,
        Err(segment) => {
            let symbol = visitor.tc.symbols.get(sym);
            let enum_name = symbol.name.clone();
            let variant = segment.ident.text;
            let error = DiagnosticKind::UnknownVariant { enum_name, variant };
            visitor.tc.error(error, segment.loc);
            None
        }
    }
}
fn check_path_type(
    visitor: &mut PatternVisitor,
    sym: EnumSymbolId,
    expected: types::TypeId,
    at: Location,
) {
    let left = visitor.tc.symbols.get(sym).ty;
    let right = visitor.tc.unwrap_type(expected).0;
    if left != right {
        let store = &visitor.tc.types;
        let left_name = display_type(store, left);
        let right_name = display_type(store, right);
        let error = DiagnosticKind::MismatchedTypes {
            left_name,
            right_name,
        };
        visitor.tc.error(error, at)
    }
}

fn find_enum_variant(
    visitor: &mut PatternVisitor,
    sym: EnumSymbolId,
    segment: ast::PathSegment,
) -> Result<Option<ir::Pattern>, ast::PathSegment> {
    let symbol = visitor.tc.symbols.get(sym);
    let ty = symbol.ty;
    let variant = symbol
        .variants
        .iter()
        .find(|v| visitor.tc.symbol_name(**v) == segment.ident.as_str());
    let Some(&variant) = variant else {
        return Err(segment);
    };
    if segment.generic_args.is_some() {
        visitor
            .tc
            .error(DiagnosticKind::UnexpectedTypeParams, segment.loc);
    }
    let variant_symbol = visitor.tc.symbols.get(variant);
    if variant_symbol.body.len() > 0 {
        visitor
            .tc
            .error(DiagnosticKind::ExpectedValueGotType, segment.loc);
        return Ok(None);
    }
    Ok(Some(ir::Pattern::Identifier(ir::Identifier {
        ty,
        loc: segment.loc,
        symbol: variant.into(),
    })))
}

fn visit_call_pattern(
    pattern: ast::CallPattern,
    visitor: &mut PatternVisitor,
    expected: types::TypeId,
) -> Option<ir::CallPattern> {
    let (callee, symbol) = visit_call_pattern_callee(visitor, pattern.path)?;
    let (expected, sub) = visitor.tc.unwrap_type(expected);
    if expected != visitor.tc.symbol_type_id(symbol) {
        visitor
            .tc
            .error(DiagnosticKind::InvalidTypeConstructor, callee.loc);
        return None;
    }
    let body = visitor.tc.symbols.get(symbol).body.clone();
    let arguments = visit_call_body(visitor, pattern.args, body, sub)?;
    Some(ir::CallPattern {
        loc: pattern.loc,
        ty: callee.ty,
        callee: (callee.loc, symbol),
        arguments,
    })
}
fn visit_call_pattern_callee(
    visitor: &mut PatternVisitor,
    ctor: ast::PathExpression,
) -> Option<(ir::Identifier, VariantSymbolId)> {
    let path = visitor.tc.visit_path_expression(ctor, PathContext::Call);
    let Some(ir::Expression::Identifier(ir::Identifier {
        symbol: SymbolId::Variant(symbol),
        loc: constructor_loc,
        ty,
    })) = path
    else {
        visitor.tc.error(DiagnosticKind::ExpectedEnum, path?.loc());
        return None;
    };
    let ident = ir::Identifier {
        ty,
        symbol: symbol.into(),
        loc: constructor_loc,
    };
    Some((ident, symbol))
}
fn visit_call_body(
    visitor: &mut PatternVisitor,
    args: Vec<ast::Pattern>,
    expected_members: Vec<MemberSymbolId>,
    substitutions: Substitutions,
) -> Option<Vec<ir::Pattern>> {
    args.into_iter()
        .zip(expected_members)
        .map(|(arg, param)| visit_call_pattern_arg(visitor, arg, param, &substitutions))
        .collect::<Option<Vec<_>>>()
}
fn visit_call_pattern_arg(
    visitor: &mut PatternVisitor,
    arg: ast::Pattern,
    param: MemberSymbolId,
    substitutions: &Substitutions,
) -> Option<ir::Pattern> {
    let raw_ty = visitor.tc.symbols.get(param).ty;
    let ty = substitutions.apply(&mut visitor.tc.types, raw_ty);
    visit_pattern(arg, visitor, ty)
}

fn visit_struct_pattern(
    pattern: ast::StructPattern,
    visitor: &mut PatternVisitor,
    expected: types::TypeId,
) -> Option<ir::Pattern> {
    let (name, symbol) = visit_struct_pattern_constructor(visitor, pattern.path)?;

    let (expected, sub) = visitor.tc.unwrap_type(expected);
    if expected != visitor.tc.symbol_type_id(symbol) {
        visitor
            .tc
            .error(DiagnosticKind::InvalidTypeConstructor, name.loc);
        return None;
    }
    let body = &visitor.tc.symbols.get(symbol).members.clone();
    let fields = visit_struct_body(visitor, pattern.fields, body, sub)?;
    Some(ir::Pattern::Struct(ir::StructPattern {
        loc: pattern.loc,
        ty: name.ty,
        name: (name.loc, symbol),
        fields,
    }))
}
fn visit_struct_pattern_constructor(
    visitor: &mut PatternVisitor,
    ctor: ast::PathExpression,
) -> Option<(ir::Identifier, StructSymbolId)> {
    let path = visitor.tc.visit_path_expression(ctor, PathContext::Struct);
    let Some(ir::Expression::Identifier(ir::Identifier {
        symbol: SymbolId::Struct(symbol),
        loc: constructor_loc,
        ty,
    })) = path
    else {
        visitor
            .tc
            .error(DiagnosticKind::ExpectedStruct, path?.loc());
        return None;
    };
    let ident = ir::Identifier {
        ty,
        symbol: symbol.into(),
        loc: constructor_loc,
    };
    Some((ident, symbol))
}

/// Visit a struct body, comparing it to its expected type
fn visit_struct_body(
    visitor: &mut PatternVisitor,
    fields: Vec<ast::StructPatternField>,
    expected_members: &[MemberSymbolId],
    substitutions: Substitutions,
) -> Option<Vec<ir::StructPatternField>> {
    fields
        .into_iter()
        .map(|f| visit_pattern_field(visitor, f, &expected_members, &substitutions))
        .collect::<Option<Vec<_>>>()
}

fn visit_pattern_field(
    visitor: &mut PatternVisitor,
    field: ast::StructPatternField,
    expected_members: &[MemberSymbolId],
    substitutions: &Substitutions,
) -> Option<ir::StructPatternField> {
    let identifier_ast = field.identifier.unwrap();
    let expected = expected_members
        .iter()
        .find(|&m| visitor.tc.symbol_name(*m) == identifier_ast.as_str());
    let Some(&expected) = expected else {
        visitor.tc.error(DiagnosticKind::InvalidMember, field.loc);
        return None;
    };
    let ty = visitor.tc.symbol_type_id(expected);
    let ty = substitutions.apply(&mut visitor.tc.types, ty);
    let identifier = ir::Identifier {
        loc: identifier_ast.loc,
        symbol: expected.into(),
        ty,
    };
    let pattern = match field.pattern {
        Some(p) => visit_pattern(p, visitor, ty)?,
        None => {
            let symbol = declare_variable(visitor, &identifier_ast, ty, false)?;
            ir::Pattern::Identifier(ir::Identifier {
                loc: identifier_ast.loc(),
                symbol: symbol.into(),
                ty,
            })
        }
    };

    Some(ir::StructPatternField {
        loc: field.loc,
        identifier,
        pattern,
    })
}

fn visit_literal_pattern(pattern: ast::LiteralPattern) -> ir::Pattern {
    use ast::LiteralPattern::*;
    match pattern {
        Boolean(b) => visit_boolean_literal(b).into(),
        Float(f) => visit_float_literal(f).into(),
        Integer(i) => visit_int_literal(i).into(),
        String(s) => visit_string_literal(s).into(),
    }
}

pub fn declare_variable(
    visitor: &mut PatternVisitor,
    identifier: &ast::Identifier,
    ty: types::TypeId,
    mutable: bool,
) -> Option<VariableSymbolId> {
    check_identifier_sanity(&mut visitor.tc, identifier);
    if mutable && visitor.is_public {
        visitor.tc.error(DiagnosticKind::PubMut, identifier.loc);
    }

    let in_current_scope = visitor.tc.current_scope().lookup(identifier.as_str());
    match in_current_scope {
        Some(symbol) => {
            let name = identifier.as_str().to_string();
            let error = DiagnosticKind::DuplicateIdentifier { name };
            visitor.tc.error(error, identifier.loc);
            visitor
                .tc
                .symbols
                .get_symbol_mut(symbol)
                .access()
                .read(identifier.loc);
            None
        }
        None => {
            let dependencies = visitor
                .dependencies
                .iter()
                .filter_map(|d| d.symbol.as_variable())
                .collect();
            let symbol: VariableSymbolId = visitor.tc.symbols.insert(VariableSymbol {
                name: identifier.as_str().to_string(),
                ty,
                public: visitor.is_public,
                mutable,
                defined_at: identifier.loc,
                dependencies,
                ..Default::default()
            });
            visitor
                .tc
                .current_scope()
                .bind(identifier.as_str().to_string(), symbol.into());
            Some(symbol)
        }
    }
}
fn check_identifier_sanity(tc: &mut TypeChecker, identifier: &ast::Identifier) {
    if identifier.as_str().contains("$") {
        tc.error(DiagnosticKind::InvalidIdentifierDollar, identifier.loc);
    }
}

fn visit_tuple_pattern(
    pattern: ast::TuplePattern,
    visitor: &mut PatternVisitor,
    expected: types::TypeId,
) -> Option<ir::TuplePattern> {
    let types::Type::Tuple(tuple) = visitor.tc.resolve(expected) else {
        visitor
            .tc
            .error(DiagnosticKind::InvalidPattern, pattern.loc);
        return None;
    };
    if tuple.elements.len() < pattern.elements.len() {
        visitor
            .tc
            .error(DiagnosticKind::InvalidPattern, pattern.loc);
    }
    let elements = pattern
        .elements
        .into_iter()
        .enumerate()
        .map(|(i, pattern)| visit_pattern(pattern, visitor, tuple.get(i)))
        .collect::<Option<Vec<_>>>()?;

    Some(ir::TuplePattern {
        ty: expected,
        loc: pattern.loc,
        elements,
    })
}

impl TypeChecker {
    pub fn visit_pattern(
        &mut self,
        pattern: ast::Pattern,
        against: &ir::Expression,
        is_public: bool,
    ) -> Option<ir::Pattern> {
        let expected_type = against.ty();
        let dependencies = self.dependencies(against).cloned().collect::<Vec<_>>();
        let mut visitor = PatternVisitor {
            tc: self,
            dependencies: &dependencies,
            is_public,
        };
        visit_pattern(pattern, &mut visitor, expected_type)
    }
}
