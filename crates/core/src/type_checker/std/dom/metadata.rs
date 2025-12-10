use crate::{
    locations::Span,
    type_checker::{
        analysis_context::{type_store::TypeStore, AnalysisContext},
        CheckData, CheckResult, SymbolData,
    },
    types::{DuckType, FunctionType, Type},
    SymbolKind,
};

pub fn dom_metadata(store: TypeStore) -> CheckResult {
    let mut analysis_context = AnalysisContext::new();
    analysis_context.type_store = store;
    analysis_context.enter_scope(Span::dummy());

    let element_trait = analysis_context.type_store.add(Type::Duck(DuckType {
        like: TypeStore::ELEMENT,
    }));
    let render_type = analysis_context
        .type_store
        .add(Type::Function(FunctionType {
            params: vec![TypeStore::STRING, element_trait],
            return_type: TypeStore::UNIT,
        }));

    analysis_context.register_symbol(SymbolData {
        name: "render".into(),
        docs: Some(
            r#"Renders a UI element into a target container in the DOM

# Example
```my-lang
render("body", <article>Content</article>)
```

In this example, the `<article>` element is rendered inside the document's body.
        "#
            .into(),
        ),
        kind: SymbolKind::Function {
            params: vec!["selector".into(), "element".into()],
            ty: render_type,
        },
        ..Default::default()
    });

    let main_scope = analysis_context
        .scopes
        .values()
        .find(|s| s.outer_id.is_none())
        .unwrap();
    let data = CheckData {
        exports: main_scope.bindings.clone(),
        expressions: analysis_context.expressions,
        tokens: analysis_context.tokens,
        dependencies: analysis_context.other_dependencies,
    };

    CheckResult {
        type_store: analysis_context.type_store,
        data,
        errors: vec![],
    }
}
