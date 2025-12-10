use crate::locations::Span;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Array(ArrayType),
    Duck(DuckType),
    Function(FunctionType),
    Listener(ListenerType),
    Map(MapType),
    Named(NamedType),
    Option(OptionType),
    Reference(ReferenceType),
    Result(ResultType),
    Signal(SignalType),
    Tuple(TupleType),
}

impl Type {
    pub fn as_span(&self) -> Span {
        match self {
            Self::Array(t) => t.span,
            Self::Duck(t) => t.span,
            Self::Function(t) => t.span,
            Self::Listener(t) => t.span,
            Self::Map(t) => t.span,
            Self::Named(t) => t.span,
            Self::Option(t) => t.span,
            Self::Reference(t) => t.span,
            Self::Result(t) => t.span,
            Self::Signal(t) => t.span,
            Self::Tuple(t) => t.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamedType {
    pub span: Span,
    pub name: String,
    pub args: Option<Vec<Type>>,
}

impl Into<Type> for NamedType {
    fn into(self) -> Type {
        Type::Named(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OptionType {
    pub span: Span,
    pub base: Option<Box<Type>>,
}

impl Into<Type> for OptionType {
    fn into(self) -> Type {
        Type::Option(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayType {
    pub span: Span,
    pub element: Option<Box<Type>>,
}

impl Into<Type> for ArrayType {
    fn into(self) -> Type {
        Type::Array(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SignalType {
    pub span: Span,
    pub inner: Box<Type>,
}

impl Into<Type> for SignalType {
    fn into(self) -> Type {
        Type::Signal(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ListenerType {
    pub span: Span,
    pub inner: Box<Type>,
}

impl Into<Type> for ListenerType {
    fn into(self) -> Type {
        Type::Listener(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReferenceType {
    pub span: Span,
    pub target: Box<Type>,
}

impl Into<Type> for ReferenceType {
    fn into(self) -> Type {
        Type::Reference(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DuckType {
    pub span: Span,
    pub like: Box<Type>,
}

impl Into<Type> for DuckType {
    fn into(self) -> Type {
        Type::Duck(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TupleType {
    pub span: Span,
    pub elements: Vec<Type>,
}

impl Into<Type> for TupleType {
    fn into(self) -> Type {
        Type::Tuple(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MapType {
    pub span: Span,
    pub key: Option<Box<Type>>,
    pub value: Option<Box<Type>>,
}

impl Into<Type> for MapType {
    fn into(self) -> Type {
        Type::Map(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResultType {
    pub span: Span,
    pub error: Option<Box<Type>>,
    pub ok: Option<Box<Type>>,
}

impl Into<Type> for ResultType {
    fn into(self) -> Type {
        Type::Result(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionType {
    pub span: Span,
    pub params: Vec<Type>,
    pub returned: Box<Type>,
}

impl Into<Type> for FunctionType {
    fn into(self) -> Type {
        Type::Function(self)
    }
}
