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
    Struct {
        fields: Vec<(String, Type)>,
    },
    Named(String), // For named/aliased types like `Person`
    Unknown,
}
