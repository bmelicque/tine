use pest::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Named(NamedType),
    Option(OptionType),
    Array(ArrayType),
    Reference(ReferenceType),
    Tuple(TupleType),
    Map(MapType),
    Result(ResultType),
    Function(FunctionType),
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamedType {
    pub span: Span<'static>,
    pub name: String,
    pub args: Option<Vec<Type>>,
}

impl Into<Type> for NamedType {
    fn into(self) -> Type {
        Type::Named(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionType {
    pub span: Span<'static>,
    pub base: Option<Box<Type>>,
}

impl Into<Type> for OptionType {
    fn into(self) -> Type {
        Type::Option(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayType {
    pub span: Span<'static>,
    pub element: Option<Box<Type>>,
}

impl Into<Type> for ArrayType {
    fn into(self) -> Type {
        Type::Array(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceType {
    pub span: Span<'static>,
    pub target: Option<Box<Type>>,
}

impl Into<Type> for ReferenceType {
    fn into(self) -> Type {
        Type::Reference(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TupleType {
    pub span: Span<'static>,
    pub elements: Vec<Type>,
}

impl Into<Type> for TupleType {
    fn into(self) -> Type {
        Type::Tuple(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapType {
    pub span: Span<'static>,
    pub key: Option<Box<Type>>,
    pub value: Option<Box<Type>>,
}

impl Into<Type> for MapType {
    fn into(self) -> Type {
        Type::Map(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResultType {
    pub span: Span<'static>,
    pub error: Option<Box<Type>>,
    pub ok: Option<Box<Type>>,
}

impl Into<Type> for ResultType {
    fn into(self) -> Type {
        Type::Result(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionType {
    pub span: Span<'static>,
    pub params: Vec<Type>,
    pub returned: Box<Type>,
}

impl Into<Type> for FunctionType {
    fn into(self) -> Type {
        Type::Function(self)
    }
}
