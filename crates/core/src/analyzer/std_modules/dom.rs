use crate::{
    type_checker::{CheckResult, TypeChecker},
    types::{FunctionType, Type},
    ModuleId, Session, SymbolData, SymbolKind, TypeStore,
};

impl Session {
    pub fn check_dom_module(&mut self, id: ModuleId) -> CheckResult {
        let mut checker = TypeChecker::new(&self, id);
        let render_type = checker.intern(Type::Function(FunctionType {
            type_params: vec![],
            params: vec![TypeStore::STRING, TypeStore::ELEMENT],
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
            ..Default::default()
        }
    }
}
