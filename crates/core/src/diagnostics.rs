use std::fmt::Display;

use crate::{ast, Location};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub loc: Location,
    pub kind: DiagnosticKind,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticKind {
    ArgumentCountMismatch {
        expected: usize,
        got: usize,
    },
    AssignmentToConstant {
        name: String,
    },
    CallbackParamCountMismatch {
        expected: usize,
        got: usize,
    },
    CannotFindModule {
        name: String,
    },
    CannotFindName {
        name: String,
    },
    CannotInferType,
    DuplicateAttribute {
        name: String,
    },
    DuplicateFieldName {
        name: String,
    },
    DuplicateIdentifier {
        name: String,
    },
    ExpectedBool {
        got: String,
    },
    ExpectedEnum {
        got: String,
    },
    ExpectedMapKey,
    ExpectedNumber {
        got: String,
    },
    ExpectedStruct {
        got: String,
    },
    ExpectedStructLikeBody,
    ExpectedToken {
        expected: Vec<String>,
    },
    ExpectedTuple {
        got: String,
    },
    ExpectedTupleLikeBody,
    ExpectedTuplePattern,
    ExpectedVariantStruct,
    ExpectedVariantTuple,
    ExpectedVariantUnit,
    ExpectedValueGotType,
    InvalidCondition {
        type_name: String,
    },
    InvalidExpression,
    InvalidIdentifierDollar,
    InvalidMember,
    InvalidPattern,
    InvalidPatternMatch {
        expected: String,
        got: String,
    },
    InvalidRootAssignee,
    InvalidTypeForOperator {
        operator: ast::BinaryOperator,
        type_name: String,
    },
    InvalidTypeConstructor,
    InvalidTypeName,
    InvalidVariantKind,
    IrrefutablePatternExpected,
    RefutablePatternExpected,
    MismatchedBranchTypes {
        expected: String,
        got: String,
    },
    MismatchedTags {
        open: String,
        close: String,
    },
    MismatchedTypes {
        left_name: String,
        right_name: String,
    },
    MissingBody,
    MissingConsequent,
    MissingConstructorName,
    MissingExpression,
    MissingField {
        name: String,
    },
    MissingFunctionName,
    MissingName,
    MissingParams,
    MissingPattern,
    MissingType,
    NegativeTupleIndex,
    NonExhaustiveMatch {
        missing: Vec<String>,
    },
    NonReactiveExpression,
    NotCallable {
        type_name: String,
    },
    NotDereferenceable {
        type_name: String,
    },
    NotIterable {
        type_name: String,
    },
    ParseError(String),
    RefToConstant {
        name: String,
    },
    ReservedName {
        name: String,
    },
    TooManyParams {
        expected: usize,
        got: usize,
    },
    TupleElementCountMismatch {
        expected: usize,
        got: usize,
    },
    UnexpectedCallback {
        expected: String,
    },
    UnexpectedModuleTree,
    UnexpectedStruct {
        expected: String,
    },
    UnexpectedToken {
        token: String,
    },
    UnexpectedTypeParams,
    UnknownMember {
        member: String,
    },
    UnknownVariant {
        variant: String,
        enum_name: String,
    },
    WrongType {
        expected: String,
        got: String,
    },
}

impl Display for DiagnosticKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArgumentCountMismatch { expected, got } => {
                write!(f, "expected {} argument(s) but got {}", expected, got)
            }
            Self::AssignmentToConstant { name } => {
                write!(f, "assignment to constant `{}`", name)
            }
            Self::CallbackParamCountMismatch { expected, got } => {
                write!(f, "expected {} param(s) but got {}", expected, got)
            }
            Self::CannotFindModule { name } => {
                write!(f, "cannot find module `{}`", name)
            }
            Self::CannotFindName { name } => {
                write!(f, "cannot find name `{}` in scope", name)
            }
            Self::CannotInferType => write!(f, "cannot infer type"),
            Self::DuplicateAttribute { name } => {
                write!(f, "duplicate attribute: `{}`", name)
            }
            Self::DuplicateFieldName { name } => {
                write!(f, "duplicate field: `{}`", name)
            }
            Self::DuplicateIdentifier { name } => {
                write!(f, "duplicate identifier: `{}`", name)
            }
            Self::ExpectedBool { got } => write!(f, "expected bool but got `{}`", got),
            Self::ExpectedEnum { got } => write!(f, "expected enum but got type `{}`", got),
            Self::ExpectedMapKey => write!(f, "expected map key but got an identifier"),
            Self::ExpectedNumber { got } => write!(f, "expected number but got `{}`", got),
            Self::ExpectedStruct { got } => write!(f, "expected struct but got type `{}`", got),
            Self::ExpectedStructLikeBody => {
                write!(f, "expected struct-like body, got tuple-like body")
            }
            Self::ExpectedToken { expected } => write!(
                f,
                "expected one of the following tokens: `{}`",
                expected.join(", ")
            ),
            Self::ExpectedTuple { got } => write!(f, "expected tuple but got type `{}`", got),
            Self::ExpectedTupleLikeBody => {
                write!(f, "expected tuple-like body but got a struct-like body")
            }
            Self::ExpectedTuplePattern => write!(f, "expected tuple pattern"),
            Self::ExpectedVariantStruct => write!(f, "expected struct variant"),
            Self::ExpectedVariantTuple => write!(f, "expected tuple variant"),
            Self::ExpectedVariantUnit => write!(f, "expected unit variant"),
            Self::ExpectedValueGotType => write!(f, "expected a value but got a type"),
            Self::InvalidCondition { type_name } => {
                write!(
                    f,
                    "condition should evaluate to a boolean, got type `{}`",
                    type_name
                )
            }
            Self::InvalidExpression => write!(f, "cannot parse expression"),
            Self::InvalidIdentifierDollar => {
                write!(f, "identifiers containing '$' are reserved for builtins")
            }
            Self::InvalidMember => write!(f, "invalid member, expected field name or integer"),
            Self::InvalidPattern => write!(f, "pattern does not match expected type"),
            Self::InvalidPatternMatch { expected, got } => {
                write!(
                    f,
                    "cannot match pattern with type `{}` against type `{}`",
                    got, expected
                )
            }
            Self::InvalidRootAssignee => write!(f, "expected identifier"),
            Self::InvalidTypeForOperator {
                operator,
                type_name,
            } => {
                write!(
                    f,
                    "invalid type `{}` for operator `{}`",
                    type_name, operator
                )
            }
            Self::InvalidTypeConstructor => write!(
                f,
                "invalid type constructor, expected a name, a map or a variant"
            ),
            Self::InvalidTypeName => write!(f, "invalid type name: name should be in PascalCase"),
            Self::InvalidVariantKind => write!(f, "invalid variant kind (unit, tuple or struct)"),
            Self::IrrefutablePatternExpected => write!(f, "irrefutable pattern expected"),
            Self::MismatchedBranchTypes { expected, got } => {
                write!(
                    f,
                    "mismatched branch types: expected `{}`, got `{}`",
                    expected, got
                )
            }
            Self::MismatchedTags { open, close } => {
                write!(
                    f,
                    "mismatched tag: got opening `{}` and closing `{}`",
                    open, close
                )
            }
            Self::MismatchedTypes {
                left_name,
                right_name,
            } => {
                write!(
                    f,
                    "mismatched types: got left type `{}` and right type `{}`",
                    left_name, right_name
                )
            }
            Self::MissingBody => write!(f, "expected function body"),
            Self::MissingConsequent => write!(f, "expected consequent"),
            Self::MissingConstructorName => write!(f, "expected constructor name"),
            Self::MissingExpression => write!(f, "expected expression"),
            Self::MissingField { name } => write!(f, "missing field `{}`", name),
            Self::MissingFunctionName => write!(f, "expected function name"),
            Self::MissingName => write!(f, "expected a name"),
            Self::MissingParams => write!(f, "expected function parameters"),
            Self::MissingPattern => write!(f, "expected pattern"),
            Self::MissingType => write!(f, "expected type"),
            Self::NegativeTupleIndex => write!(f, "tuple index cannot be negative"),
            Self::NonExhaustiveMatch { missing } => {
                let missing = if missing.len() > 2 {
                    format!(
                        "{}, {}, and {} more...",
                        missing[0],
                        missing[1],
                        missing.len() - 2
                    )
                } else if missing.len() == 2 {
                    format!("{} and {}", missing[0], missing[1])
                } else if missing.len() == 1 {
                    format!("{}", missing[0])
                } else {
                    unreachable!()
                };
                write!(f, "non-exhaustive match: missing variants {}", missing)
            }
            Self::NonReactiveExpression => write!(f, "expected a reactive expression"),
            Self::NotCallable { type_name } => {
                write!(f, "type `{}` cannot be called", type_name)
            }
            Self::NotDereferenceable { type_name } => {
                write!(f, "type `{}` cannot be dereferenced", type_name)
            }
            Self::NotIterable { type_name } => {
                write!(f, "type `{}` cannot be iterated over", type_name)
            }
            Self::ParseError(msg) => {
                write!(f, "parse error: {}", msg)
            }
            Self::RefutablePatternExpected => {
                write!(f, "expected refutable pattern")
            }
            Self::RefToConstant { name } => write!(
                f,
                "cannot take mutable reference of constant variable `{}`",
                name
            ),
            Self::ReservedName { name } => {
                write!(f, "invalid identifier: `{}` is a reserved keyword", name)
            }
            Self::TooManyParams { expected, got } => {
                write!(f, "expected {} parameter(s) but got {}", expected, got)
            }
            Self::TupleElementCountMismatch { expected, got } => {
                write!(f, "expected {} element(s) but got {}", expected, got)
            }
            Self::UnexpectedCallback { expected } => {
                write!(f, "expected type `{}` but got a callback", expected)
            }
            Self::UnexpectedStruct { expected } => {
                write!(f, "expected type `{}` but got a struct", expected)
            }
            Self::UnexpectedToken { token } => {
                write!(f, "unexpected token: {}", token)
            }
            Self::UnexpectedTypeParams => write!(f, "unexpected type parameters"),
            Self::UnknownMember { member } => {
                write!(f, "unknown member `{}`", member)
            }
            Self::UnexpectedModuleTree => write!(f, "unexpected module tree"),
            Self::UnknownVariant { variant, enum_name } => {
                write!(f, "unknown variant `{}` for enum `{}`", variant, enum_name)
            }
            Self::WrongType { expected, got } => {
                write!(f, "wrong type: expected `{}`, got `{}`", expected, got)
            }
        }
    }
}
