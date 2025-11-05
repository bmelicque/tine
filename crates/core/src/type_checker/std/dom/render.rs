use crate::{type_checker::std::dom::utils::element_type, types};

pub fn render() -> types::FunctionType {
    types::FunctionType {
        params: vec![types::Type::String, element_type()],
        return_type: Box::new(types::Type::Void),
    }
}
