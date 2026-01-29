use crate::Location;

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
    pub fn loc(&self) -> Location {
        match self {
            Self::Array(t) => t.loc,
            Self::Duck(t) => t.loc,
            Self::Function(t) => t.loc,
            Self::Listener(t) => t.loc,
            Self::Map(t) => t.loc,
            Self::Named(t) => t.loc,
            Self::Option(t) => t.loc,
            Self::Reference(t) => t.loc,
            Self::Result(t) => t.loc,
            Self::Signal(t) => t.loc,
            Self::Tuple(t) => t.loc,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamedType {
    pub loc: Location,
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
    pub loc: Location,
    pub base: Option<Box<Type>>,
}

impl Into<Type> for OptionType {
    fn into(self) -> Type {
        Type::Option(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayType {
    pub loc: Location,
    pub element: Option<Box<Type>>,
}

impl Into<Type> for ArrayType {
    fn into(self) -> Type {
        Type::Array(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SignalType {
    pub loc: Location,
    pub inner: Box<Type>,
}

impl Into<Type> for SignalType {
    fn into(self) -> Type {
        Type::Signal(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ListenerType {
    pub loc: Location,
    pub inner: Box<Type>,
}

impl Into<Type> for ListenerType {
    fn into(self) -> Type {
        Type::Listener(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReferenceType {
    pub loc: Location,
    pub target: Box<Type>,
}

impl Into<Type> for ReferenceType {
    fn into(self) -> Type {
        Type::Reference(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DuckType {
    pub loc: Location,
    pub like: Box<Type>,
}

impl Into<Type> for DuckType {
    fn into(self) -> Type {
        Type::Duck(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TupleType {
    pub loc: Location,
    pub elements: Vec<Type>,
}

impl Into<Type> for TupleType {
    fn into(self) -> Type {
        Type::Tuple(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MapType {
    pub loc: Location,
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
    pub loc: Location,
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
    pub loc: Location,
    pub params: Vec<Type>,
    pub returned: Box<Type>,
}

impl Into<Type> for FunctionType {
    fn into(self) -> Type {
        Type::Function(self)
    }
}
