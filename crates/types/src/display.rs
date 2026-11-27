use crate::{
    store::TypeStore,
    types::{StructType, TraitMethod, TraitType, Type, TypeId, TypeRef},
};

pub fn display_type(store: &TypeStore, ty: TypeId) -> String {
    if let Some(name) = store.get_alias(ty) {
        return name.clone();
    }
    display_raw_type(store, ty)
}

pub fn display_raw_type(store: &TypeStore, ty: TypeId) -> String {
    match &store.get(ty) {
        Type::Boolean => "bool".into(),
        Type::Enum(t) => t
            .variants
            .iter()
            .map(|variant| format!("{}({})", variant.name, display_type(store, variant.def)))
            .collect::<Vec<_>>()
            .join(" | "),
        Type::Float => "float".into(),
        Type::Function(t) => {
            let params = display_params(store, &t.params);
            let return_type = display_return_type(store, t.return_type);
            format!("fn({}){}", params, return_type)
        }
        Type::Generic(_) => "generic".into(),
        Type::Listener(t) => {
            format!("Computed<{}>", display_type(store, t.inner))
        }
        Type::Integer => "int".into(),
        Type::Param(t) => t.name.clone(),
        Type::Placeholder(_) => "{unknown}".into(),
        Type::Ref(t) => display_type_ref(store, t),
        Type::SelfType => "Self".into(),
        Type::Signal(t) => {
            format!("${}", display_type(store, t.inner))
        }
        Type::String => "str".into(),
        Type::Struct(t) => display_struct_type(store, t),
        Type::Trait(t) => display_trait(store, t),
        Type::Tuple(t) => {
            let elements = t
                .elements
                .iter()
                .map(|e| display_type(store, *e))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({})", elements)
        }
        Type::Unit => "()".into(),
        Type::Unknown => "unknown".into(),
    }
}

fn display_type_ref(store: &TypeStore, t: &TypeRef) -> String {
    if t.inner == TypeStore::ARRAY {
        return format!("{}[]", display_type(store, t.args[0]));
    }
    if t.inner == store.well_known("Result") {
        assert_eq!(t.args.len(), 2);
        let ok = display_type(store, t.args[0]);
        let error = display_type(store, t.args[1]);
        return format!("{error}!{ok}");
    }

    let args = display_type_args(store, &t.args);
    format!("{}{}", display_type(store, t.inner), args)
}
fn display_type_args(store: &TypeStore, args: &[TypeId]) -> String {
    if args.len() == 0 {
        return "".into();
    }

    let args = args
        .iter()
        .map(|arg| display_type(store, *arg))
        .collect::<Vec<_>>()
        .join(", ");
    format!("<{args}>")
}

fn display_struct_type(store: &TypeStore, struct_: &StructType) -> String {
    let fields = struct_
        .fields
        .iter()
        .map(|field| format!("{}: {}", field.name, display_type(store, field.def)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{ {} }}", fields)
}

fn display_params(store: &TypeStore, params: &[TypeId]) -> String {
    params
        .iter()
        .map(|p| display_type(store, *p))
        .collect::<Vec<_>>()
        .join(", ")
}

fn display_return_type(store: &TypeStore, ty: TypeId) -> String {
    match ty {
        TypeStore::UNIT => "".to_string(),
        ty => format!(" -> {}", display_type(store, ty)),
    }
}

fn display_trait(store: &TypeStore, t: &TraitType) -> String {
    let methods = t
        .methods
        .iter()
        .map(|method| display_trait_method(store, method))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{ {} }}", methods)
}

fn display_trait_method(store: &TypeStore, method: &TraitMethod) -> String {
    let name = &method.name;
    let Type::Function(ty) = store.get(method.def) else {
        panic!()
    };
    let params = display_params(store, &ty.params);
    let return_type = display_return_type(store, ty.return_type);
    format!("fn {}({}){}", name, params, return_type)
}
