use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Array(ArrayType),
    Boolean,
    Duck(DuckType),
    Dynamic, // Represents a type that will have to be inferred later
    Enum(EnumType),
    Function(FunctionType),
    Generic(GenericType), // Represents a generic type parameter
    Listener(ListenerType),
    Map(MapType),
    Named(NamedType),
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

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayType {
    pub element: Box<Type>,
}

impl Into<Type> for ArrayType {
    fn into(self) -> Type {
        Type::Array(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DuckType {
    pub like: Box<Type>,
}

impl Into<Type> for DuckType {
    fn into(self) -> Type {
        Type::Duck(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumType {
    pub variants: Vec<Variant>,
}

impl Into<Type> for EnumType {
    fn into(self) -> Type {
        Type::Enum(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub name: String,
    pub def: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionType {
    pub params: Vec<Type>,
    pub return_type: Box<Type>,
}

impl Into<Type> for FunctionType {
    fn into(self) -> Type {
        Type::Function(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericType {
    pub name: String,
}

impl Into<Type> for GenericType {
    fn into(self) -> Type {
        Type::Generic(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapType {
    pub key: Box<Type>,
    pub value: Box<Type>,
}

impl Into<Type> for MapType {
    fn into(self) -> Type {
        Type::Map(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamedType {
    pub name: String,
    pub args: Vec<Type>,
}

impl Into<Type> for NamedType {
    fn into(self) -> Type {
        Type::Named(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionType {
    pub some: Box<Type>,
}

impl Into<Type> for OptionType {
    fn into(self) -> Type {
        Type::Option(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SignalType {
    pub inner: Box<Type>,
}

impl Into<Type> for SignalType {
    fn into(self) -> Type {
        Type::Signal(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListenerType {
    pub inner: Box<Type>,
}

impl Into<Type> for ListenerType {
    fn into(self) -> Type {
        Type::Listener(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceType {
    pub target: Box<Type>,
}

impl Into<Type> for ReferenceType {
    fn into(self) -> Type {
        Type::Reference(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResultType {
    pub ok: Box<Type>,
    pub error: Option<Box<Type>>,
}

impl Into<Type> for ResultType {
    fn into(self) -> Type {
        Type::Result(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructType {
    pub fields: Vec<StructField>,
}

impl Into<Type> for StructType {
    fn into(self) -> Type {
        Type::Struct(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub def: Type,
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitType {
    pub methods: Vec<TraitMethod>,
}

impl Into<Type> for TraitType {
    fn into(self) -> Type {
        Type::Trait(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitMethod {
    pub name: String,
    pub def: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TupleType {
    pub elements: Vec<Type>,
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
            Type::Generic(ty) => write!(f, "{}", ty.name),
            Type::Listener(ty) => write!(f, "@{}", ty.inner),
            Type::Map(ty) => write!(f, "{}#{}", ty.key, ty.value),
            Type::Named(ty) => {
                if ty.args.len() == 0 {
                    return write!(f, "{}", ty.name);
                }

                let args_str = ty
                    .args
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{}[{}]", ty.name, args_str)
            }
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
