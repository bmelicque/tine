use std::collections::HashMap;

use crate::{
    type_checker::{CheckResult, TypeChecker},
    types::{DuckType, FunctionType, Type},
    ModuleId, Session, SymbolData, SymbolKind, TypeStore,
};

impl Session {
    pub fn check_dom_module(&mut self, id: ModuleId) -> CheckResult {
        let mut checker = TypeChecker::new(&self, id);
        let element_trait = checker.intern(Type::Duck(DuckType {
            like: TypeStore::ELEMENT,
        }));
        let render_type = checker.intern(Type::Function(FunctionType {
            params: vec![TypeStore::STRING, element_trait],
            return_type: TypeStore::UNIT,
        }));

        checker.ctx.register_symbol(SymbolData {
            name: "render".into(),
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
            kind: SymbolKind::Function {
                param_names: vec!["selector".into(), "element".into()],
            },
            ..Default::default()
        });

        let main_scope = &checker.ctx.scopes[0];

        CheckResult {
            symbols: checker.ctx.symbols,
            exports: main_scope.bindings.clone(),
            expressions: HashMap::new(),
            dependencies: HashMap::new(),
            diagnostics: vec![],
        }
    }
}
