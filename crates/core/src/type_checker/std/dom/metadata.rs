use crate::{
    type_checker::{
        analysis_context::{type_store::TypeStore, AnalysisContext, ModuleMetadata},
        SymbolData,
    },
    types::{DuckType, FunctionType, StructType, Type},
    utils::dummy_span,
};

pub fn dom_metadata() -> ModuleMetadata {
    let mut analysis_context = AnalysisContext::new();
    analysis_context.enter_scope(dummy_span());

    let element_type = analysis_context.type_store.add(Type::Struct(StructType {
        id: analysis_context.type_store.get_next_id(),
        // TODO: define fields
        fields: vec![],
    }));
    analysis_context
        .type_store
        .add_alias(element_type, "Element".into());
    let element_trait = analysis_context
        .type_store
        .add(Type::Duck(DuckType { like: element_type }));
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

    analysis_context.into()
}
