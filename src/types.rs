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
        fields: Vec<(String, Type)>,
    },
    Tuple(Vec<Type>),
    Generic {
        name: String,
        params: Vec<Type>,
    },
    Map {
        key: Box<Type>,
        value: Box<Type>,
    },
    Option(Box<Type>),
    Result {
        error: Option<Box<Type>>,
        ok: Box<Type>,
    },

    Unknown,
}
