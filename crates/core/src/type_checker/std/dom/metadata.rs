use crate::{
    type_checker::{
        analysis_context::{type_store::TypeStore, AnalysisContext},
        CheckData, CheckResult, SymbolData,
    },
    types::{DuckType, FunctionType, Type},
    utils::dummy_span,
};

pub fn dom_metadata(store: TypeStore) -> CheckResult {
    let mut analysis_context = AnalysisContext::new();
    analysis_context.type_store = store;
    analysis_context.enter_scope(dummy_span());

    let element_trait = analysis_context.type_store.add(Type::Duck(DuckType {
        like: TypeStore::ELEMENT,
    }));
    let render_type = analysis_context
        .type_store
        .add(Type::Function(FunctionType {
            params: vec![TypeStore::STRING, element_trait],
            return_type: TypeStore::VOID,
        }));

    analysis_context.register_symbol(SymbolData::pure(
        "render".to_string(),
        render_type,
        dummy_span(),
    ));

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
