use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    String,
    Number,
    Boolean,
    Void,
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    Named {
        name: String,
        args: Vec<Type>,
    },
    Array(Box<Type>),
    Struct {
        fields: Vec<StructField>,
    },
    Sum {
        variants: Vec<SumVariant>,
    },
    Trait {
        methods: Vec<TraitMethod>,
    },
    Tuple(Vec<Type>),
    Map {
        key: Box<Type>,
        value: Box<Type>,
    },
    Option(Box<Type>),
    Reference(Box<Type>),
    Result {
        error: Option<Box<Type>>,
        ok: Box<Type>,
    },
    Dynamic,         // Represents a type that will have to be inferred later
    Generic(String), // Represents a generic type parameter
    SelfType,        // Represents the current type in a method context
    Unit,

    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub def: Type,
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SumVariant {
    pub name: String,
    pub def: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitMethod {
    pub name: String,
    pub def: Type,
}

impl Type {
    pub fn is_assignable_to(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::Unknown, _) => true,
            (_, Type::Unknown) => true,
            (_, _) => self == other,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::String => write!(f, "string"),
            Type::Number => write!(f, "number"),
            Type::Boolean => write!(f, "boolean"),
            Type::Void => write!(f, "void"),
            Type::Function {
                params,
                return_type,
            } => {
                let params_str = params
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "({}) => {}", params_str, return_type)
            }
            Type::Named { name, args } => {
                if args.len() > 0 {
                    let args_str = args
                        .iter()
                        .map(|a| a.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    write!(f, "{}[{}]", name, args_str)
                } else {
                    write!(f, "{}", name)
                }
            }
            Type::Array(inner) => write!(f, "[]{}", inner),
            Type::Struct { fields } => {
                let fields_str = fields
                    .iter()
                    .map(|field| format!("{}: {}", field.name, field.def))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{{ {} }}", fields_str)
            }
            Type::Sum { variants } => {
                let variants_str = variants
                    .iter()
                    .map(|variant| format!("{} {{{}}}", variant.name, variant.def))
                    .collect::<Vec<_>>()
                    .join(" | ");
                write!(f, "{}", variants_str)
            }
            Type::Trait { methods } => {
                let methods_str = methods
                    .iter()
                    .map(|method| format!("{}: {}", method.name, method.def))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, ".{{ {} }}", methods_str)
            }
            Type::Tuple(types) => {
                let types_str = types
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "({})", types_str)
            }
            Type::Map { key, value } => write!(f, "{}#{}", key, value),
            Type::Option(inner) => write!(f, "?{}", inner),
            Type::Reference(inner) => write!(f, "&{}", inner),
            Type::Result { error, ok } => {
                if let Some(error) = error {
                    write!(f, "{}!{}", error, ok)
                } else {
                    write!(f, "!{}", ok)
                }
            }
            Type::Dynamic => write!(f, "any"),
            Type::Generic(name) => write!(f, "{}", name),
            Type::SelfType => write!(f, "Self"),
            Type::Unknown => write!(f, "unknown"),
            Type::Unit => write!(f, "()"),
        }
    }
}
