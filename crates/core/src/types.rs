use std::fmt;

pub type TypeId = u32;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Array(ArrayType),
    Boolean,
    Duck(DuckType),
    Dynamic, // Represents a type that will have to be inferred later
    Enum(EnumType),
    Function(FunctionType),
    Generic(GenericType),
    Param(TypeParam), // Represents a generic type parameter
    Listener(ListenerType),
    Map(MapType),
    Number,
    Option(OptionType),
    Reference(ReferenceType),
    Result(ResultType),
    SelfType, // Represents the current type in a method context
    Signal(SignalType),
    String,
    Struct(StructType),
    Trait(TraitType),
    Tuple(TupleType),
    Unit,
    Unknown,
    Void,
}

impl Type {
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
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericType {
    /// When creating the type, should create each of the Type::Param and store the id
    pub params: Vec<TypeId>,
    /// The Type definition of this generic type.
    ///
    /// It contains the `TypeParam`s that are also referenced in order in the `params` field.
    /// Those will be substituted at some point with the concrete types.
    pub definition: TypeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayType {
    pub element: TypeId,
}

impl Into<Type> for ArrayType {
    fn into(self) -> Type {
        Type::Array(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DuckType {
    pub like: TypeId,
}

impl Into<Type> for DuckType {
    fn into(self) -> Type {
        Type::Duck(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumType {
    /// This `id` is used to differentiate between enums with identical definitions,
    /// like `A :: True|False` and `B :: True|False`.
    ///
    /// In case this is a canonicalized generic, this id refers to the generic definition.
    pub id: TypeId,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionType {
    pub params: Vec<TypeId>,
    pub return_type: TypeId,
}

impl Into<Type> for FunctionType {
    fn into(self) -> Type {
        Type::Function(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeParam {
    pub name: String,
    /// Index of the param in the param list of the associated generic type
    pub idx: usize,
}

impl Into<Type> for TypeParam {
    fn into(self) -> Type {
        Type::Param(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MapType {
    pub key: TypeId,
    pub value: TypeId,
}

impl Into<Type> for MapType {
    fn into(self) -> Type {
        Type::Map(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OptionType {
    pub some: TypeId,
}

impl Into<Type> for OptionType {
    fn into(self) -> Type {
        Type::Option(self)
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
pub struct ReferenceType {
    pub target: TypeId,
}

impl Into<Type> for ReferenceType {
    fn into(self) -> Type {
        Type::Reference(self)
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructType {
    /// This `id` is used to differentiate between identically structured types,
    /// like `A :: (value number)` and `B :: (value number)`.
    ///
    /// In case this is a canonicalized generic, this id refers to the generic definition.
    pub id: TypeId,
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
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitType {
    pub methods: Vec<TraitMethod>,
}

impl Into<Type> for TraitType {
    fn into(self) -> Type {
        Type::Trait(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitMethod {
    pub name: String,
    pub def: TypeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TupleType {
    pub elements: Vec<TypeId>,
}

impl Into<Type> for TupleType {
    fn into(self) -> Type {
        Type::Tuple(self)
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Array(ty) => write!(f, "[]{}", ty.element),
            Type::Boolean => write!(f, "boolean"),
            Type::Duck(ty) => write!(f, "~{}", ty.like),
            Type::Dynamic => write!(f, "any"),
            Type::Enum(ty) => {
                let variants_str = ty
                    .variants
                    .iter()
                    .map(|variant| format!("{} {{{}}}", variant.name, variant.def))
                    .collect::<Vec<_>>()
                    .join(" | ");
                write!(f, "{}", variants_str)
            }
            Type::Function(ty) => {
                let params_str = ty
                    .params
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "({}) => {}", params_str, ty.return_type)
            }
            Type::Generic(_) => todo!(),
            Type::Param(ty) => write!(f, "{}", ty.name),
            Type::Listener(ty) => write!(f, "@{}", ty.inner),
            Type::Map(ty) => write!(f, "{}#{}", ty.key, ty.value),
            Type::Number => write!(f, "number"),
            Type::Option(ty) => write!(f, "?{}", ty.some),
            Type::Reference(ty) => write!(f, "&{}", ty.target),
            Type::Result(ty) => {
                if let Some(error) = &ty.error {
                    write!(f, "{}!{}", error, ty.ok)
                } else {
                    write!(f, "!{}", ty.ok)
                }
            }
            Type::SelfType => write!(f, "Self"),
            Type::Signal(ty) => write!(f, "${}", ty.inner),
            Type::String => write!(f, "string"),
            Type::Struct(ty) => {
                let fields_str = ty
                    .fields
                    .iter()
                    .map(|field| format!("{}: {}", field.name, field.def))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{{ {} }}", fields_str)
            }
            Type::Trait(ty) => {
                let methods_str = ty
                    .methods
                    .iter()
                    .map(|method| format!("{}: {}", method.name, method.def))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, ".{{ {} }}", methods_str)
            }
            Type::Tuple(ty) => {
                let types_str = ty
                    .elements
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "({})", types_str)
            }
            Type::Unknown => write!(f, "unknown"),
            Type::Unit => write!(f, "()"),
            Type::Void => write!(f, "void"),
        }
    }
}
