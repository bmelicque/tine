use std::collections::HashMap;

use tine_common::module_path::ModuleId;
use tine_symbols::symbols::*;
use tine_types::{store::TypeStore, types};

use crate::TypeChecker;

pub fn check_dom_module(module: ModuleId, tc: &mut TypeChecker) {
    let mut exports = HashMap::new();

    let render_type = tc.intern(types::FunctionType {
        type_params: vec![],
        params: vec![TypeStore::STRING, TypeStore::ELEMENT],
        return_type: TypeStore::UNIT,
    });

    let render_name = "render".to_string();
    let render_symbol = tc.symbols.insert::<FunctionSymbolId>(FunctionSymbol {
        name: render_name.clone(),
        docs: Some(
            r#"Renders a UI element into a target container in the DOM

# Example
```tine
render("body", <article>Content</article>)
```

In this example, the `<article>` element is rendered inside the document's body.
        "#
            .into(),
        ),
        ty: render_type,
        param_names: vec!["selector".into(), "element".into()],
        ..Default::default()
    });
    exports.insert(render_name, render_symbol.into());

    tc.add_exports(module, exports);
}
