use tine_core::{
    types::{FunctionType, GenericType, Type, TypeId},
    SymbolKind, TypeStore,
};

use crate::{tokens::ServerSymbol, Backend};

impl Backend {
    pub fn display_signature(&self, symbol: &ServerSymbol) -> String {
        let session = self.session.read().unwrap();
        let name = &symbol.0.name;
        let ty = symbol.0.ty;
        match &symbol.0.kind {
            SymbolKind::Function { .. } => self.display_function_symbol(&symbol),
            SymbolKind::PrimitiveType { .. } => session.types().display_raw_type(ty),
            SymbolKind::TypeAlias => {
                format!("type {} = {}", name, session.types().display_raw_type(ty))
            }
            SymbolKind::Struct { .. } => {
                format!("struct {} {}", name, session.types().display_raw_type(ty))
            }
            SymbolKind::Enum { .. } => {
                format!("enum {} {}", name, session.types().display_raw_type(ty))
            }
            SymbolKind::Value { mutable } => {
                let ty = session.types().display_type(ty);
                let operator = if *mutable { "var" } else { "const" };
                format!("{} {} {}", operator, name, ty)
            }
            SymbolKind::Member { owner } => {
                let owner_name = &owner.borrow().name;
                let member_name = name;
                let displayed_type = session.types().display_type(ty);
                format!("{}.{} {}", owner_name, member_name, displayed_type)
            }
            SymbolKind::Method { .. } => self.display_method_symbol(symbol),
            SymbolKind::Constructor { owner, .. } => {
                let owner_name = &owner.borrow().name;
                // TODO: FIXME:
                format!("{}.{}", owner_name, name)
            }
        }
    }

    fn display_function_symbol(&self, symbol: &ServerSymbol) -> String {
        let SymbolKind::Function { param_names } = &symbol.0.kind else {
            panic!()
        };
        let session = self.session.read().unwrap();
        let name = &symbol.0.name;
        let ty = symbol.0.ty;

        let params = self.display_function_params(ty, param_names);
        let return_type = self.get_return_type(ty);
        match return_type {
            TypeStore::UNIT => format!("fn {}({})", name, params),
            _ => format!(
                "fn {}({}) {}",
                name,
                params,
                session.types().display_type(return_type)
            ),
        }
    }

    fn display_method_symbol(&self, symbol: &ServerSymbol) -> String {
        let SymbolKind::Method {
            owner,
            owner_args,
            has_receiver,
            param_names,
        } = &symbol.0.kind
        else {
            panic!()
        };
        let session = self.session.read().unwrap();
        let name = &symbol.0.name;
        let ty = symbol.0.ty;
        let receiver = if *has_receiver {
            let owner_name = &owner.borrow().name;
            let args = owner_args
                .iter()
                .map(|arg| session.types().display_type(*arg))
                .collect::<Vec<_>>()
                .join(", ");
            match args.len() {
                0 => format!("({}) ", owner_name),
                _ => format!("({}<{}>) ", owner_name, args),
            }
        } else {
            String::new()
        };

        let method_name = name;
        let params = self.display_function_params(ty, param_names);
        let return_type = match self.get_return_type(ty) {
            TypeStore::UNIT => String::new(),
            t => format!(" {}", session.types().display_type(t)),
        };
        format!("fn {}{}({}){}", receiver, method_name, params, return_type)
    }

    fn display_function_params(&self, ty: TypeId, names: &Vec<String>) -> String {
        let session = self.session.read().unwrap();
        let store = session.types();
        let f = match store.get(ty) {
            Type::Function(f) => f,
            Type::Generic(g) => match store.get(g.definition) {
                Type::Function(f) => f,
                _ => panic!("expected function type"),
            },
            _ => panic!("expected function type"),
        };
        f.params
            .iter()
            .zip(names)
            .map(|(ty, name)| {
                let ty = store.display_type(*ty);
                format!("{} {}", name, ty)
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Get the return type of a function or generic function
    fn get_return_type(&self, ty: TypeId) -> TypeId {
        let session = self.session.read().unwrap();
        let ty = session.types().get(ty).to_owned();
        match ty {
            Type::Function(FunctionType {
                ref return_type, ..
            }) => *return_type,
            Type::Generic(GenericType { ref definition, .. }) => {
                let definition = session.types().get(*definition).to_owned();
                match definition {
                    Type::Function(FunctionType {
                        ref return_type, ..
                    }) => *return_type,
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }
}
