use crate::types;

pub fn node_type() -> types::Type {
    types::Type::Duck(types::DuckType {
        like: Box::new(types::Type::Named(types::NamedType {
            name: "Node".to_string(),
            args: vec![],
        })),
    })
}
pub fn node_array() -> types::Type {
    types::Type::Array(types::ArrayType {
        element: Box::new(node_type()),
    })
}
pub fn node_option() -> types::Type {
    types::Type::Option(types::OptionType {
        some: Box::new(node_type()),
    })
}

pub fn element_type() -> types::Type {
    types::Type::Duck(types::DuckType {
        like: Box::new(types::Type::Named(types::NamedType {
            name: "Element".to_string(),
            args: vec![],
        })),
    })
}
pub fn element_option() -> types::Type {
    types::Type::Option(types::OptionType {
        some: Box::new(element_type()),
    })
}

pub fn document_type() -> types::Type {
    types::Type::Duck(types::DuckType {
        like: Box::new(types::Type::Named(types::NamedType {
            name: "Document".to_string(),
            args: vec![],
        })),
    })
}
pub fn document_option() -> types::Type {
    types::Type::Option(types::OptionType {
        some: Box::new(document_type()),
    })
}

pub fn string_option() -> types::Type {
    types::Type::Option(types::OptionType {
        some: Box::new(types::Type::String),
    })
}

pub fn make_field(name: &str, ty: types::Type) -> types::StructField {
    types::StructField {
        name: name.to_string(),
        def: ty,
        optional: false,
    }
}
pub fn make_method(
    name: &str,
    params: Vec<types::Type>,
    return_type: types::Type,
) -> types::StructField {
    types::StructField {
        name: name.to_string(),
        def: types::Type::Function(types::FunctionType {
            params,
            return_type: Box::new(return_type),
        }),
        optional: false,
    }
}
