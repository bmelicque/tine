use crate::store::TypeStore;

pub type TypeId = u32;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Boolean,
    Dynamic, // Represents a type that will have to be inferred later
    Enum(EnumType),
    Float,
    Function(FunctionType),
    Generic(GenericDef),
    Integer,
    Param(TypeParam), // Represents a generic type parameter
    Placeholder(Placeholder),
    Listener(ListenerType),
    Ref(TypeRef),
    Result(ResultType),
    SelfType, // Represents the current type in a method context
    Signal(SignalType),
    String,
    Struct(StructType),
    Trait(TraitType),
    Tuple(TupleType),
    Unit,
    Unknown,
}

impl Type {
    pub fn as_struct(&self) -> Option<&StructType> {
        match self {
            Self::Struct(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_enum(&self) -> Option<&EnumType> {
        match self {
            Self::Enum(e) => Some(e),
            _ => None,
        }
    }

    pub fn as_function(&self) -> Option<&FunctionType> {
        match self {
            Self::Function(f) => Some(f),
            _ => None,
        }
    }

    pub fn as_trait(&self) -> Option<&TraitType> {
        match self {
            Self::Trait(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_ref(&self) -> Option<&TypeRef> {
        match self {
            Self::Ref(t) => Some(t),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&TypeRef> {
        match self {
            Self::Ref(t) if t.inner == TypeStore::ARRAY => Some(t),
            _ => None,
        }
    }

    pub fn as_tuple(&self) -> Option<&TupleType> {
        match self {
            Self::Tuple(t) => Some(t),
            _ => None,
        }
    }

    /// Get a reactive's inner value
    pub fn unwrapped(&self) -> Option<TypeId> {
        match self {
            Self::Listener(l) => Some(l.inner),
            Self::Signal(s) => Some(s.inner),
            _ => None,
        }
    }

    pub fn is_reactive(&self) -> bool {
        match self {
            Self::Listener(_) | Self::Signal(_) => true,
            _ => false,
        }
    }

    pub fn is_unresolved(&self) -> bool {
        *self == Type::Dynamic
    }

    pub fn is_unknown(&self) -> bool {
        *self == Type::Unknown
    }

    pub fn is_function(&self) -> bool {
        match self {
            Self::Function(_) => true,
            _ => false,
        }
    }

    pub fn is_generic(&self) -> bool {
        match self {
            Self::Enum(e) => e.params.len() > 0,
            Self::Function(f) => f.params.len() > 0,
            Self::Generic(_) => true,
            Self::Struct(s) => s.params.len() > 0,
            Self::Trait(t) => t.params.len() > 0,
            _ => false,
        }
    }

    pub fn as_params(&self) -> Option<&[TypeParam]> {
        match self {
            Self::Enum(e) => Some(&e.params),
            Self::Function(f) => Some(&f.type_params),
            Self::Generic(g) => Some(&g.params),
            Self::Struct(s) => Some(&s.params),
            Self::Trait(t) => Some(&t.params),
            _ => None,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct EnumType {
    /// This `id` is used to differentiate between enums with identical definitions,
    /// like `A :: True|False` and `B :: True|False`.
    ///
    /// In case this is a canonicalized generic, this id refers to the generic definition.
    pub id: TypeId,
    pub params: Vec<TypeParam>,
    pub variants: Vec<Variant>,
}

impl Into<Type> for EnumType {
    fn into(self) -> Type {
        Type::Enum(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Variant {
    pub name: String,
    pub def: TypeId,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct FunctionType {
    pub type_params: Vec<TypeParam>,
    pub params: Vec<TypeId>,
    pub return_type: TypeId,
}

impl Into<Type> for FunctionType {
    fn into(self) -> Type {
        Type::Function(self)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct TypeParam {
    pub name: String,
    pub id: TypeId,
}
impl Into<Type> for TypeParam {
    fn into(self) -> Type {
        Type::Param(self)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Placeholder {
    pub id: TypeId,
}
impl Into<Type> for Placeholder {
    fn into(self) -> Type {
        Type::Placeholder(self)
    }
}

/// Used for generic type aliases
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericDef {
    pub def: TypeId,
    pub params: Vec<TypeParam>,
}
impl Into<Type> for GenericDef {
    fn into(self) -> Type {
        Type::Generic(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeRef {
    pub inner: TypeId,
    pub args: Vec<TypeId>,
}
impl TypeRef {
    pub fn new(inner: TypeId, args: Vec<TypeId>) -> Self {
        Self { inner, args }
    }
}
impl Into<Type> for TypeRef {
    fn into(self) -> Type {
        Type::Ref(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SignalType {
    pub inner: TypeId,
}

impl Into<Type> for SignalType {
    fn into(self) -> Type {
        Type::Signal(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ListenerType {
    pub inner: TypeId,
}

impl Into<Type> for ListenerType {
    fn into(self) -> Type {
        Type::Listener(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResultType {
    pub ok: TypeId,
    pub error: Option<TypeId>,
}

impl Into<Type> for ResultType {
    fn into(self) -> Type {
        Type::Result(self)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct StructType {
    /// This `id` is used to differentiate between identically structured types,
    /// like `A :: (value number)` and `B :: (value number)`.
    ///
    /// In case this is a canonicalized generic, this id refers to the generic definition.
    pub id: TypeId,
    pub params: Vec<TypeParam>,
    pub fields: Vec<StructField>,
}

impl Into<Type> for StructType {
    fn into(self) -> Type {
        Type::Struct(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructField {
    pub name: String,
    pub def: TypeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitType {
    pub params: Vec<TypeParam>,
    pub methods: Vec<TraitMethod>,
}

impl Into<Type> for TraitType {
    fn into(self) -> Type {
        Type::Trait(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitMethod {
    pub self_type: Option<TypeParam>,
    pub name: String,
    pub def: TypeId,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct TupleType {
    pub params: Vec<TypeParam>,
    pub elements: Vec<TypeId>,
}
impl TupleType {
    pub fn get(&self, index: usize) -> TypeId {
        self.elements
            .get(index)
            .copied()
            .unwrap_or(TypeStore::UNKNOWN)
    }
}
impl Into<Type> for TupleType {
    fn into(self) -> Type {
        Type::Tuple(self)
    }
}
