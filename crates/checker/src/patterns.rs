use std::collections::VecDeque;

use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};
use tine_ir::{self as ir, Typed};
use tine_macros::EnumFrom;
use tine_symbols::{symbols::*, table::*};
use tine_types::{
    store::{display_type, TypeStore},
    types,
};

use crate::{
    expressions::expressions::{
        visit_boolean_literal, visit_float_literal, visit_int_literal, visit_string_literal,
    },
    substitutions::Substitutions,
    PathContext, TypeChecker,
};

#[derive(Debug, Default)]
pub struct LoweredPattern {
    pub test: Option<ir::Expression>,
    pub decls: Vec<ir::VariableDeclaration>,
}
impl LoweredPattern {
    pub fn merge(a: Self, b: Self) -> Self {
        let test = match (a.test, b.test) {
            (Some(a), Some(b)) => Some(ir::Expression::Binary(ir::BinaryExpression {
                loc: Location::merge(a.loc(), b.loc()),
                left: Box::new(a),
                right: Box::new(b),
                op: ir::BinaryOperator::LAnd,
                ty: TypeStore::BOOLEAN,
            })),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        let mut decls = a.decls;
        decls.extend(b.decls);
        LoweredPattern { test, decls }
    }
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard,
    Identifier(IdentifierPattern),
    Literal(LiteralPattern),

    Constructor(ConstructorPattern),
    Struct(StructPattern),
    Tuple(TuplePattern),
}

pub fn display_pattern(pattern: &Pattern, symbols: &SymbolTable) -> String {
    match pattern {
        Pattern::Wildcard => "_".to_string(),
        Pattern::Identifier(i) => i.name.to_string(),
        Pattern::Constructor(c) => {
            let name = symbols.get_symbol(c.identifier.symbol).name();
            match &c.arg {
                Some(arg) => format!("{}({})", name, display_pattern(&arg, symbols)),
                None => name.to_string(),
            }
        }
        Pattern::Literal(l) => match l {
            LiteralPattern::Bool(l) => format!("{}", l.value),
            LiteralPattern::Int(l) => format!("{}", l.value),
            LiteralPattern::Float(l) => format!("{}", l.value),
            LiteralPattern::String(l) => format!("\"{}\"", l.value),
        },
        Pattern::Struct(s) => {
            let fields = s
                .fields
                .iter()
                .map(|f| {
                    format!(
                        "{}: {}",
                        symbols.get_symbol(f.0.symbol).name(),
                        display_pattern(&f.1, symbols)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");

            format!("{{{}}}", fields)
        }
        Pattern::Tuple(t) => {
            let items = t
                .items
                .iter()
                .map(|f| display_pattern(&f.1, symbols))
                .collect::<Vec<_>>()
                .join(", ");

            format!("({})", items)
        }
    }
}

pub fn lower_pattern(pattern: Pattern, value: ir::Expression) -> LoweredPattern {
    match pattern {
        Pattern::Literal(p) => lower_literal_pattern(p, value),
        Pattern::Identifier(p) => lower_identifier_pattern(p, value),
        Pattern::Wildcard => LoweredPattern::default(),
        Pattern::Constructor(p) => lower_constructor_pattern(p, value),
        Pattern::Struct(p) => lower_struct_pattern(p, value),
        Pattern::Tuple(p) => lower_tuple_pattern(p, value),
    }
}
fn lower_literal_pattern(pattern: LiteralPattern, value: ir::Expression) -> LoweredPattern {
    let test = ir::Expression::Binary(ir::BinaryExpression {
        loc: Location::merge(pattern.loc(), value.loc()),
        left: Box::new(value),
        right: Box::new(pattern.into()),
        op: ir::BinaryOperator::EqEq,
        ty: TypeStore::BOOLEAN,
    });
    LoweredPattern {
        test: Some(test),
        decls: vec![],
    }
}
fn lower_identifier_pattern(pattern: IdentifierPattern, value: ir::Expression) -> LoweredPattern {
    let decl = ir::VariableDeclaration {
        loc: Location::merge(pattern.identifier.loc, value.loc()),
        mutable: pattern.mut_kw,
        symbol: pattern.identifier.symbol.as_variable().unwrap(),
        value,
    };

    LoweredPattern {
        test: None,
        decls: vec![decl],
    }
}
fn lower_constructor_pattern(pattern: ConstructorPattern, value: ir::Expression) -> LoweredPattern {
    let test = Some(ir::Expression::TypeMatch(ir::TypeMatch {
        loc: Location::merge(pattern.identifier.loc, value.loc()),
        expr: Box::new(value.clone()),
        variant: pattern.identifier.symbol.as_variant().unwrap(),
    }));
    let lowered = LoweredPattern {
        test,
        decls: vec![],
    };
    match pattern.arg {
        Some(arg) => {
            let arg = lower_pattern(*arg, value);
            LoweredPattern::merge(lowered, arg)
        }
        None => lowered,
    }
}
fn lower_struct_pattern(pattern: StructPattern, value: ir::Expression) -> LoweredPattern {
    pattern
        .fields
        .into_iter()
        .fold(LoweredPattern::default(), |acc, field| {
            LoweredPattern::merge(acc, lower_pattern_field(field, value.clone()))
        })
}
fn lower_tuple_pattern(pattern: TuplePattern, value: ir::Expression) -> LoweredPattern {
    pattern
        .items
        .into_iter()
        .fold(LoweredPattern::default(), |acc, item| {
            LoweredPattern::merge(acc, lower_pattern_field(item, value.clone()))
        })
}
fn lower_pattern_field(field: PatternField, value: ir::Expression) -> LoweredPattern {
    let value = ir::Expression::Member(ir::MemberExpression {
        loc: value.loc(),
        object: Some(Box::new(value)),
        member: (field.0.loc, field.0.symbol.as_member().unwrap()),
        ty: TypeStore::UNKNOWN,
    });
    lower_pattern(field.1, value)
}

macro_rules! impl_pattern {
    ($name:ident, $variant:ident, $as_name:ident) => {
        impl From<$name> for Pattern {
            fn from(p: $name) -> Self {
                Self::$variant(p)
            }
        }

        impl Pattern {
            pub fn $as_name(&self) -> Option<&$name> {
                match self {
                    Self::$variant(p) => Some(p),
                    _ => None,
                }
            }
        }
    };
}

#[derive(Debug, Clone)]
pub struct IdentifierPattern {
    pub pub_kw: bool,
    pub mut_kw: bool,
    pub name: String,
    pub identifier: ir::Identifier,
}
impl_pattern!(IdentifierPattern, Identifier, as_identifier);

#[derive(Debug, EnumFrom, Clone)]
pub enum LiteralPattern {
    Bool(ir::BooleanLiteral),
    Int(ir::IntLiteral),
    Float(ir::FloatLiteral),
    String(ir::StringLiteral),
}
impl_pattern!(LiteralPattern, Literal, as_literal);

impl From<LiteralPattern> for ir::Expression {
    fn from(p: LiteralPattern) -> Self {
        use ir::Expression::*;
        use LiteralPattern::*;
        match p {
            Bool(b) => BooleanLiteral(b),
            Int(i) => IntLiteral(i),
            Float(f) => FloatLiteral(f),
            String(s) => StringLiteral(s),
        }
    }
}
impl LiteralPattern {
    pub fn loc(&self) -> Location {
        match self {
            Self::Bool(b) => b.loc,
            Self::Int(i) => i.loc,
            Self::Float(f) => f.loc,
            Self::String(s) => s.loc,
        }
    }

    pub fn as_bool(&self) -> Option<&ir::BooleanLiteral> {
        match self {
            Self::Bool(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<&ir::FloatLiteral> {
        match self {
            Self::Float(f) => Some(f),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<&ir::IntLiteral> {
        match self {
            Self::Int(i) => Some(i),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&ir::StringLiteral> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct StructPattern {
    pub fields: Vec<PatternField>,
}
impl_pattern!(StructPattern, Struct, as_struct);

#[derive(Debug, Clone)]
pub struct PatternField(pub ir::Identifier, pub Pattern);

#[derive(Debug, Clone)]
pub struct ConstructorPattern {
    pub identifier: ir::Identifier,
    pub arg: Option<Box<Pattern>>,
}
impl_pattern!(ConstructorPattern, Constructor, as_constructor);

#[derive(Debug, Default, Clone)]
pub struct TuplePattern {
    pub items: Vec<PatternField>,
}
impl_pattern!(TuplePattern, Tuple, as_tuple);

pub(super) struct PatternVisitor<'deps, 'tc> {
    pub(super) is_declaration: bool,
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
    let identifier: ir::Identifier = if visitor.is_declaration {
        let symbol = declare_variable(visitor, &pattern.identifier, expected, pattern.mutable)?;
        ir::Identifier {
            loc: pattern.loc(),
            symbol: symbol.into(),
            ty: expected,
        }
    } else {
        visitor.tc.visit_identifier(pattern.identifier)?
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
    let identifier = field.identifier.unwrap();
    let expected = expected_members
        .iter()
        .find(|&m| visitor.tc.symbol_name(*m) == identifier.as_str());
    let Some(&expected) = expected else {
        visitor.tc.error(DiagnosticKind::InvalidMember, field.loc);
        return None;
    };
    let ty = visitor.tc.symbol_type_id(expected);
    let ty = substitutions.apply(&mut visitor.tc.types, ty);
    let identifier = ir::Identifier {
        loc: identifier.loc,
        symbol: expected.into(),
        ty,
    };
    let pattern = match field.pattern {
        Some(p) => Some(visit_pattern(p, visitor, ty)?),
        None => None,
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
    visitor.tc.check_identifier_sanity(&identifier);
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
        is_declaration: bool,
        is_public: bool,
    ) -> Option<ir::Pattern> {
        let expected_type = against.ty();
        let dependencies = self.dependencies(against).cloned().collect::<Vec<_>>();
        let mut visitor = PatternVisitor {
            tc: self,
            dependencies: &dependencies,
            is_declaration,
            is_public,
        };
        visit_pattern(pattern, &mut visitor, expected_type)
    }
}
