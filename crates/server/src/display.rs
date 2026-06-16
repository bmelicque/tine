use tine_core::{
    display_raw_type, display_type,
    types::{FunctionType, Type, TypeId},
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
            SymbolKind::PrimitiveType { .. } => display_raw_type(&session.types(), ty),
            SymbolKind::TypeAlias => {
                format!("type {} = {}", name, display_raw_type(&session.types(), ty))
            }
            SymbolKind::Struct { .. } => {
                format!("struct {} {}", name, display_raw_type(&session.types(), ty))
            }
            SymbolKind::Enum { .. } => {
                format!("enum {} {}", name, display_raw_type(&session.types(), ty))
            }
            SymbolKind::Value { mutable } => {
                let ty = display_type(&session.types(), ty);
                let operator = if *mutable { "var" } else { "const" };
                format!("{} {} {}", operator, name, ty)
            }
            SymbolKind::Member { owner } => {
                let owner_name = &owner.borrow().name;
                let member_name = name;
                let displayed_type = display_type(&session.types(), ty);
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
                display_type(&session.types(), return_type)
            ),
        }
    }

    fn display_method_symbol(&self, symbol: &ServerSymbol) -> String {
        let SymbolKind::Method {
            owner,
            owner_args,
            receiver,
            param_names,
        } = &symbol.0.kind
        else {
            panic!()
        };
        let session = self.session.read().unwrap();
        let name = &symbol.0.name;
        let ty = symbol.0.ty;
        let receiver = if receiver.is_static() {
            String::new()
        } else {
            let owner_name = &owner.borrow().name;
            let args = owner_args
                .iter()
                .map(|arg| display_type(&session.types(), *arg.1))
                .collect::<Vec<_>>()
                .join(", ");
            match args.len() {
                0 => format!("({}) ", owner_name),
                _ => format!("({}<{}>) ", owner_name, args),
            }
        };

        let method_name = name;
        let params = self.display_function_params(ty, param_names);
        let return_type = match self.get_return_type(ty) {
            TypeStore::UNIT => String::new(),
            t => format!(" {}", display_type(&session.types(), t)),
        };
        format!("fn {}{}({}){}", receiver, method_name, params, return_type)
    }

    fn display_function_params(&self, ty: TypeId, names: &Vec<String>) -> String {
        let session = self.session.read().unwrap();
        let store = session.types();
        let Type::Function(f) = store.get(ty) else {
            panic!("expected function type")
        };
        f.params
            .iter()
            .zip(names)
            .map(|(ty, name)| {
                let ty = display_type(&store, *ty);
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
            _ => panic!(),
        }
    }
}
