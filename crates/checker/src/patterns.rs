use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};
use tine_ir::{self as ir, Typed};
use tine_macros::EnumFrom;
use tine_symbols::{symbols::*, table::*};
use tine_types::{store::TypeStore, types};

use crate::{
    expressions::expressions::{
        visit_boolean_literal, visit_float_literal, visit_int_literal, visit_string_literal,
    },
    TypeChecker,
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
        object: Box::new(value),
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
) -> Option<Pattern> {
    use tine_ast::Pattern::*;
    match pattern {
        Constructor(c) => visit_constructor_pattern(c, visitor, expected),
        Identifier(i) => visit_identifier_pattern(i, visitor, expected, false),
        Invalid(_) => None,
        MutIdentifier(i) => visit_identifier_pattern(i.identifier, visitor, expected, true),
        Literal(l) => Some(visit_literal_pattern(l).into()),
        Tuple(t) => visit_tuple_pattern(t, visitor, expected).map(Into::into),
    }
}

fn visit_constructor_pattern(
    pattern: ast::ConstructorPattern,
    visitor: &mut PatternVisitor,
    expected: types::TypeId,
) -> Option<Pattern> {
    use tine_ast::Constructor::*;
    let got_name = match pattern.constructor {
        Invalid(_) => return None,
        Map(_) => unimplemented!(),
        Named(n) => n.name,
        Variant(v) => v.variant_name?,
    };

    let symbol = visitor.tc.resolve_type_symbol(expected)?;
    match symbol {
        TypeSymbolId::Struct(s) => {
            let body = &visitor.tc.symbols.get(s).body.clone();
            visit_type_symbol_body(visitor, pattern.body, body)
        }
        TypeSymbolId::Enum(e) => {
            let variants = &visitor.tc.symbols.get(e).variants;
            let variant = variants
                .iter()
                .find(|v| got_name.as_str() == visitor.tc.symbol_name(**v));
            let Some(variant) = variant else {
                visitor
                    .tc
                    .error(DiagnosticKind::InvalidPattern, pattern.loc);
                return None;
            };
            let identifier = ir::Identifier {
                loc: got_name.loc,
                symbol: (*variant).into(),
                ty: visitor.tc.symbol_type_id(e),
            };
            let body = visitor.tc.symbols.get(*variant).body.as_ref().cloned();
            let arg = body
                .and_then(|b| visit_type_symbol_body(visitor, pattern.body, &b))
                .map(Into::into);
            Some(Pattern::Constructor(ConstructorPattern { identifier, arg }))
        }
        _ => panic!(),
    }
}
fn visit_type_symbol_body(
    visitor: &mut PatternVisitor,
    pattern: Option<ast::ConstructorPatternBody>,
    expected: &TypeSymbolBody,
) -> Option<Pattern> {
    use tine_ast::ConstructorPatternBody::*;
    match (expected, pattern?) {
        (TypeSymbolBody::Struct(expected), Struct(s)) => visit_struct_body(visitor, s, expected),
        (TypeSymbolBody::Tuple(expected), Tuple(t)) => visit_tuple_body(visitor, t, expected),
        (_, p) => {
            visitor.tc.error(DiagnosticKind::InvalidPattern, p.loc());
            None
        }
    }
}

/// Visit a struct body, comparing it to its expected type
fn visit_struct_body(
    visitor: &mut PatternVisitor,
    pattern: ast::StructPatternBody,
    expected: &[(String, MemberSymbolId)],
) -> Option<Pattern> {
    let fields = pattern
        .fields
        .into_iter()
        .map(|f| visit_pattern_field(visitor, f, &expected))
        .collect::<Option<Vec<_>>>()?;
    Some(StructPattern { fields }.into())
}

fn visit_tuple_body(
    visitor: &mut PatternVisitor,
    pattern: ast::TuplePattern,
    expected: &[MemberSymbolId],
) -> Option<Pattern> {
    if expected.len() < pattern.elements.len() {
        visitor
            .tc
            .error(DiagnosticKind::InvalidPattern, pattern.loc);
    }
    let items: Vec<PatternField> = pattern
        .elements
        .into_iter()
        .enumerate()
        .map(|(i, e)| {
            let symbol_id = *expected.get(i)?;
            let symbol = visitor.tc.symbols.get(symbol_id);
            let identifier = ir::Identifier {
                loc: e.loc(),
                symbol: symbol_id.into(),
                ty: symbol.ty,
            };
            let pattern = visit_pattern(e, visitor, symbol.ty);
            Some(PatternField(identifier, pattern?))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(TuplePattern { items }.into())
}

fn visit_pattern_field(
    visitor: &mut PatternVisitor,
    field: ast::StructPatternField,
    expected: &[(String, MemberSymbolId)],
) -> Option<PatternField> {
    let identifier = match field.identifier {
        Some(ast::FieldPatternIdentifier::Const(i)) => i,
        Some(ast::FieldPatternIdentifier::Mut(i)) => i.identifier,
        None => panic!(),
    };
    let expected = expected.iter().find(|&(n, _)| n == identifier.as_str());
    let Some((_, expected)) = expected else {
        visitor.tc.error(DiagnosticKind::InvalidMember, field.loc);
        return None;
    };
    let ty = visitor.tc.symbol_type_id(*expected);
    let identifier = ir::Identifier {
        loc: identifier.loc,
        symbol: (*expected).into(),
        ty,
    };
    let pattern = visit_pattern(field.pattern?, visitor, ty)?;
    Some(PatternField(identifier, pattern))
}

fn visit_literal_pattern(pattern: ast::LiteralPattern) -> LiteralPattern {
    use tine_ast::LiteralPattern::*;
    match pattern {
        Boolean(b) => visit_boolean_literal(b).into(),
        Float(f) => visit_float_literal(f).into(),
        Integer(i) => visit_int_literal(i).into(),
        String(s) => visit_string_literal(s).into(),
    }
}

fn visit_identifier_pattern(
    pattern: ast::Identifier,
    visitor: &mut PatternVisitor,
    expected: types::TypeId,
    mutable: bool,
) -> Option<Pattern> {
    use Pattern::*;
    let name = pattern.as_str().to_string();
    let identifier: ir::Identifier = if visitor.is_declaration {
        let symbol = declare_variable(visitor, &pattern, expected, mutable)?;
        ir::Identifier {
            loc: pattern.loc(),
            symbol: symbol.into(),
            ty: expected,
        }
    } else {
        visitor.tc.visit_identifier(pattern)?
    };

    Some(Identifier(IdentifierPattern {
        mut_kw: mutable,
        pub_kw: visitor.is_public,
        name,
        identifier,
    }))
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
) -> Option<TuplePattern> {
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
    let items = pattern
        .elements
        .into_iter()
        .enumerate()
        .map(|(i, pattern)| {
            // TODO: against type
            let expected = tuple.elements.get(i).copied().unwrap_or(TypeStore::UNKNOWN);
            let symbol_id = visitor.tc.symbols.insert::<MemberSymbolId>(MemberSymbol {
                name: format!("_{i}"),
                // FIXME:
                owner: StructSymbolId::from_index(0).into(),
                defined_at: pattern.loc(),
                ty: expected,
                ..Default::default()
            });
            let identifier = ir::Identifier {
                loc: pattern.loc(),
                symbol: symbol_id.into(),
                ty: expected,
            };
            let pattern = visit_pattern(pattern, visitor, expected)?;

            Some(PatternField(identifier, pattern))
        })
        .collect::<Option<Vec<_>>>()?;

    Some(TuplePattern { items })
}

impl TypeChecker {
    pub fn visit_pattern(
        &mut self,
        pattern: ast::Pattern,
        against: &ir::Expression,
        is_declaration: bool,
        is_public: bool,
    ) -> Option<Pattern> {
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
