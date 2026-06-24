use enum_from_derive::EnumFrom;

use crate::{
    ast, ir,
    type_checker::{
        expressions::expressions::{
            visit_boolean_literal, visit_float_literal, visit_int_literal, visit_string_literal,
        },
        TypeChecker,
    },
    types, DiagnosticKind, Location, SymbolData, SymbolKind, SymbolRef, TypeStore, TypeSymbolBody,
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

pub enum Pattern {
    Literal(LiteralPattern),
    Identifier(IdentifierPattern),
    Wildcard,

    Constructor(ConstructorPattern),
    Struct(StructPattern),
    Tuple(TuplePattern),
}
impl From<ir::Identifier> for Pattern {
    fn from(value: ir::Identifier) -> Self {
        Self::Identifier(value.into())
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
        symbol: pattern.identifier.symbol,
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
        constructor: pattern.identifier.symbol,
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
        member: field.0,
        ty: TypeStore::UNKNOWN,
    });
    lower_pattern(field.1, value)
}

pub struct IdentifierPattern {
    pub mut_kw: bool,
    pub identifier: ir::Identifier,
}
impl From<IdentifierPattern> for Pattern {
    fn from(p: IdentifierPattern) -> Self {
        Self::Identifier(p)
    }
}
impl From<ir::Identifier> for IdentifierPattern {
    fn from(value: ir::Identifier) -> Self {
        Self {
            mut_kw: false,
            identifier: value,
        }
    }
}

#[derive(Debug, EnumFrom)]
pub enum LiteralPattern {
    Bool(ir::BooleanLiteral),
    Int(ir::IntLiteral),
    Float(ir::FloatLiteral),
    String(ir::StringLiteral),
}
impl From<LiteralPattern> for Pattern {
    fn from(p: LiteralPattern) -> Self {
        Self::Literal(p)
    }
}
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
            LiteralPattern::Bool(b) => b.loc,
            LiteralPattern::Int(i) => i.loc,
            LiteralPattern::Float(f) => f.loc,
            LiteralPattern::String(s) => s.loc,
        }
    }
}

pub struct StructPattern {
    pub fields: Vec<PatternField>,
}
impl From<StructPattern> for Pattern {
    fn from(p: StructPattern) -> Self {
        Self::Struct(p)
    }
}

pub struct PatternField(ir::Identifier, Pattern);

pub struct ConstructorPattern {
    pub identifier: ir::Identifier,
    pub arg: Option<Box<Pattern>>,
}

pub struct TuplePattern {
    pub items: Vec<PatternField>,
}
impl From<TuplePattern> for Pattern {
    fn from(p: TuplePattern) -> Self {
        Self::Tuple(p)
    }
}

struct PatternVisitor<'deps, 'tc, 'ctx> {
    is_declaration: bool,
    /// All the dependencies of the expression the pattern is matched against
    dependencies: &'deps [ir::Identifier],
    type_checker: &'tc mut TypeChecker<'ctx>,
}

fn visit_pattern(
    pattern: ast::Pattern,
    visitor: &mut PatternVisitor,
    expected: types::TypeId,
) -> Option<Pattern> {
    use ast::Pattern::*;
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
    use ast::Constructor::*;
    let got_name = match pattern.constructor {
        Invalid(_) => return None,
        Map(_) => unimplemented!(),
        Named(n) => n.name,
        Variant(v) => v.variant_name?,
    };

    let symbol = visitor.type_checker.resolve_type_symbol(expected)?;
    let kind = symbol.borrow().kind.clone();
    match kind {
        SymbolKind::Struct { body, .. } => visit_type_symbol_body(visitor, pattern.body, body),
        SymbolKind::Enum { variants, .. } => {
            let variant = variants.iter().find(|v| got_name.as_str() == &v.as_name());
            let Some(variant) = variant else {
                visitor
                    .type_checker
                    .error(DiagnosticKind::InvalidPattern, pattern.loc);
                return None;
            };
            let identifier = ir::Identifier {
                loc: got_name.loc,
                symbol: variant.clone(),
            };
            let arg = variant
                .as_type_body()
                .and_then(|b| visit_type_symbol_body(visitor, pattern.body, b))
                .map(Into::into);
            Some(Pattern::Constructor(ConstructorPattern { identifier, arg }))
        }
        _ => unreachable!(),
    }
}
fn visit_type_symbol_body(
    visitor: &mut PatternVisitor,
    pattern: Option<ast::ConstructorPatternBody>,
    expected: TypeSymbolBody,
) -> Option<Pattern> {
    use ast::ConstructorPatternBody::*;
    match (expected, pattern?) {
        (TypeSymbolBody::Struct(expected), Struct(s)) => {
            let fields = s
                .fields
                .into_iter()
                .map(|f| visit_pattern_field(visitor, f, &expected))
                .collect::<Option<Vec<_>>>()?;
            Some(StructPattern { fields }.into())
        }
        (TypeSymbolBody::Tuple(expected), Tuple(t)) => {
            if expected.len() < t.elements.len() {
                visitor
                    .type_checker
                    .error(DiagnosticKind::InvalidPattern, t.loc);
            }
            let items: Vec<PatternField> = t
                .elements
                .into_iter()
                .enumerate()
                .map(|(i, e)| {
                    let symbol = expected.get(i);
                    let identifier = ir::Identifier {
                        loc: e.loc(),
                        symbol: symbol?.clone(),
                    };
                    let pattern = visit_pattern(
                        e,
                        visitor,
                        symbol.map_or(TypeStore::UNKNOWN, |e| e.as_type()),
                    );
                    Some(PatternField(identifier, pattern?))
                })
                .collect::<Option<Vec<_>>>()?;
            Some(TuplePattern { items }.into())
        }
        (_, p) => {
            visitor
                .type_checker
                .error(DiagnosticKind::InvalidPattern, p.loc());
            None
        }
    }
}
fn visit_pattern_field(
    visitor: &mut PatternVisitor,
    field: ast::StructPatternField,
    expected: &[(String, SymbolRef)],
) -> Option<PatternField> {
    let identifier = match field.identifier {
        Some(ast::FieldPatternIdentifier::Const(i)) => i.0,
        Some(ast::FieldPatternIdentifier::Mut(i)) => i.identifier.0,
        None => panic!(),
    };
    let expected = expected.iter().find(|&(n, _)| n == identifier.as_str());
    let Some((_, expected)) = expected else {
        visitor
            .type_checker
            .error(DiagnosticKind::InvalidMember, field.loc);
        return None;
    };
    let identifier = ir::Identifier {
        loc: identifier.loc,
        symbol: expected.clone(),
    };
    let pattern = visit_pattern(field.pattern?, visitor, expected.as_type())?;
    Some(PatternField(identifier, pattern))
}

fn visit_literal_pattern(pattern: ast::LiteralPattern) -> LiteralPattern {
    use ast::LiteralPattern::*;
    match pattern {
        Boolean(b) => visit_boolean_literal(b).into(),
        Float(f) => visit_float_literal(f).into(),
        Integer(i) => visit_int_literal(i).into(),
        String(s) => visit_string_literal(s).into(),
    }
}

fn visit_identifier_pattern(
    pattern: ast::IdentifierPattern,
    visitor: &mut PatternVisitor,
    expected: types::TypeId,
    mutable: bool,
) -> Option<Pattern> {
    use Pattern::*;
    if pattern.as_str() == "_" {
        return Some(Wildcard);
    }

    let identifier: ir::Identifier = if visitor.is_declaration {
        let symbol = declare_variable(visitor, &pattern.0, expected, mutable)?;
        ir::Identifier {
            loc: pattern.loc(),
            symbol,
        }
    } else {
        visitor.type_checker.visit_identifier(pattern.0)?
    };

    Some(Identifier(IdentifierPattern {
        mut_kw: mutable,
        identifier,
    }))
}

fn declare_variable(
    visitor: &mut PatternVisitor,
    identifier: &ast::Identifier,
    ty: types::TypeId,
    mutable: bool,
) -> Option<SymbolRef> {
    visitor.type_checker.check_identifier_sanity(&identifier);

    match visitor
        .type_checker
        .ctx
        .find_in_current_scope(identifier.as_str())
    {
        Some(symbol) => {
            let error = DiagnosticKind::DuplicateIdentifier {
                name: identifier.as_str().to_string(),
            };
            visitor.type_checker.error(error, identifier.loc);
            symbol.borrow().access.read(identifier.loc);
            None
        }
        None => {
            let dependencies = visitor
                .dependencies
                .iter()
                .map(|d| d.symbol.clone())
                .collect();
            let symbol = visitor.type_checker.ctx.register_symbol(SymbolData {
                name: identifier.as_str().to_string(),
                ty,
                kind: SymbolKind::Value { mutable },
                defined_at: identifier.loc,
                dependencies,
                ..Default::default()
            });
            Some(symbol)
        }
    }
}

fn visit_tuple_pattern(
    pattern: ast::TuplePattern,
    visitor: &mut PatternVisitor,
    expected: types::TypeId,
) -> Option<TuplePattern> {
    let tuple = match visitor.type_checker.resolve(expected) {
        types::Type::Tuple(t) => t,
        _ => {
            visitor
                .type_checker
                .error(DiagnosticKind::InvalidPattern, pattern.loc);
            return None;
        }
    };
    if tuple.elements.len() < pattern.elements.len() {
        visitor
            .type_checker
            .error(DiagnosticKind::InvalidPattern, pattern.loc);
    }
    let items = pattern
        .elements
        .into_iter()
        .enumerate()
        .map(|(i, pattern)| {
            // TODO: against type
            let identifier = ir::Identifier {
                loc: pattern.loc(),
                symbol: SymbolRef::new(format!("_{i}"), pattern.loc()),
            };
            let expected = tuple.elements.get(i).copied().unwrap_or(TypeStore::UNKNOWN);
            let pattern = visit_pattern(pattern, visitor, expected)?;

            Some(PatternField(identifier, pattern))
        })
        .collect::<Option<Vec<_>>>()?;

    Some(TuplePattern { items })
}

impl TypeChecker<'_> {
    pub fn visit_pattern(
        &mut self,
        pattern: ast::Pattern,
        against: &ir::Expression,
        is_declaration: bool,
    ) -> Option<Pattern> {
        let expected_type = against.ty();
        let dependencies = against.dependencies().cloned().collect::<Vec<_>>();
        let mut visitor = PatternVisitor {
            type_checker: self,
            dependencies: &dependencies,
            is_declaration,
        };
        visit_pattern(pattern, &mut visitor, expected_type)
    }
}
