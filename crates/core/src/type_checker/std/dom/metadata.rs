use std::{collections::HashMap, rc::Rc};

use crate::{
    type_checker::{
        analysis_context::{ModuleMetadata, VariableHandle},
        std::dom::render::render,
        VariableData,
    },
    utils::dummy_span,
};

pub fn dom_metadata() -> ModuleMetadata {
    let render = VariableData::new(
        "render".to_string(),
        Rc::new(render().into()),
        false,
        dummy_span(),
        Vec::new(),
    );

    let exports = vec![VariableHandle::new(render).readonly()];

    ModuleMetadata {
        exports,
        types: HashMap::new(),
        dependencies: HashMap::new(),
    }
}
