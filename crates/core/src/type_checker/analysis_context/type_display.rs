use crate::{types::*, TypeStore};

pub fn display_type(store: &TypeStore, ty: TypeId) -> String {
    if let Some(name) = store.get_alias(ty) {
        return name.clone();
    }
    display_raw_type(store, ty)
}

pub fn display_raw_type(store: &TypeStore, ty: TypeId) -> String {
    match &store.get(ty) {
        Type::Array(t) => {
            format!("{}[]", display_type(store, t.element))
        }
        Type::Boolean => "bool".into(),
        Type::Dynamic => "dynamic".into(),
        Type::Enum(t) => t
            .variants
            .iter()
            .map(|variant| format!("{}({})", variant.name, display_type(store, variant.def)))
            .collect::<Vec<_>>()
            .join(" | "),
        Type::Float => "float".into(),
        Type::Function(t) => {
            let params = t
                .params
                .iter()
                .map(|p| display_type(store, *p))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({}) => {}", params, display_type(store, t.return_type))
        }
        Type::Generic(_) => "generic".into(),
        Type::Listener(t) => {
            format!("@{}", display_type(store, t.inner))
        }
        Type::Map(t) => {
            format!(
                "{}#{}",
                display_type(store, t.key),
                display_type(store, t.value)
            )
        }
        Type::Integer => "int".into(),
        Type::Option(t) => {
            format!("?{}", display_type(store, t.some))
        }
        Type::Param(t) => t.name.clone(),
        Type::Ref(t) => {
            let args = t
                .args
                .iter()
                .map(|arg| display_type(store, *arg))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}<{}>", display_type(store, t.inner), args)
        }
        Type::Result(t) => {
            if let Some(error) = &t.error {
                format!(
                    "{}!{}",
                    display_type(store, *error),
                    display_type(store, t.ok)
                )
            } else {
                format!("!{}", display_type(store, t.ok))
            }
        }
        Type::SelfType => "Self".into(),
        Type::Signal(t) => {
            format!("${}", display_type(store, t.inner))
        }
        Type::String => "str".into(),
        Type::Struct(t) => {
            let fields = t
                .fields
                .iter()
                .map(|field| format!("{} {}", field.name, display_type(store, field.def)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {} }}", fields)
        }
        Type::Trait(t) => {
            let methods = t
                .methods
                .iter()
                .map(|method| format!("{} {}", method.name, display_type(store, method.def)))
                .collect::<Vec<_>>()
                .join(", ");
            format!(".({})", methods)
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_type_primitives() {
        let store = TypeStore::new();
        assert_eq!(display_type(&store, TypeStore::INTEGER), "int");
        assert_eq!(display_type(&store, TypeStore::STRING), "str");
        assert_eq!(display_type(&store, TypeStore::BOOLEAN), "bool");
        assert_eq!(display_type(&store, TypeStore::UNIT), "()");
    }

    #[test]
    fn test_display_type_array() {
        let mut store = TypeStore::new();
        let array_type = Type::Array(ArrayType {
            element: TypeStore::INTEGER,
        });
        let array_id = store.add(array_type);
        assert_eq!(display_type(&store, array_id), "int[]");
    }

    #[test]
    fn test_display_type_function() {
        let mut store = TypeStore::new();
        let fn_type = Type::Function(FunctionType {
            type_params: vec![],
            params: vec![TypeStore::STRING, TypeStore::INTEGER],
            return_type: TypeStore::BOOLEAN,
        });
        let fn_id = store.add(fn_type);
        assert_eq!(display_type(&store, fn_id), "(str, int) => bool");
    }

    #[test]
    fn test_add_alias() {
        let mut store = TypeStore::new();
        store.add_alias(TypeStore::INTEGER, "MyNumber".to_string());
        assert_eq!(display_type(&store, TypeStore::INTEGER), "MyNumber");
    }
}
