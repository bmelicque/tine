use std::collections::HashMap;

use crate::{
    type_checker::{analysis_context::ModuleMetadata, std::dom::render::render, Symbol},
    utils::dummy_span,
};

pub fn dom_context() -> ModuleMetadata {
    let render = Symbol::new(
        "render".to_string(),
        render().into(),
        false,
        dummy_span(),
        Vec::new(),
    );

    let mut exports = HashMap::new();
    exports.insert(0, render);

    ModuleMetadata {
        exports,
        types: HashMap::new(),
        dependencies: HashMap::new(),
    }
}
