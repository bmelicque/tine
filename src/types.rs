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
    Named(String),
    Array(Box<Type>),
    Struct {
        fields: Vec<StructField>,
    },
    Sum {
        variants: Vec<SumVariant>,
    },
    Tuple(Vec<Type>),
    Generic {
        name: String,
        args: Vec<Type>,
    },
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
    Dynamic, // Represents a type that will have to be inferred later

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
