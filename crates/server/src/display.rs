use tine_symbols::symbols::*;
use tine_types::{
    store::{display_raw_type, display_type, TypeStore},
    types::{FunctionType, Type, TypeId},
};

use crate::Backend;

impl Backend {
    pub fn display_signature(&self, symbol: SymbolId) -> String {
        let symbols = self.symbols();
        use SymbolId::*;
        match symbol {
            Function(s) => self.display_function_symbol(s),
            Primitive(s) => display_raw_type(&self.types(), self.symbols().get(s).ty),
            TypeAlias(s) => {
                let s = symbols.get(s);
                format!(
                    "type {} = {}",
                    s.name,
                    display_raw_type(&self.types(), s.ty)
                )
            }
            Struct(s) => {
                let s = symbols.get(s);
                format!(
                    "struct {} {}",
                    s.name,
                    display_raw_type(&self.types(), s.ty)
                )
            }
            Enum(s) => self.display_enum_symbol(s),
            Variable(s) => {
                let s = symbols.get(s);
                let operator = if s.mutable { "let mut" } else { "let" };
                let ty = display_type(&self.types(), s.ty);
                format!("{} {}: {}", operator, s.name, ty)
            }
            Member(s) => {
                let s = symbols.get(s);
                let pub_ = if s.public { "pub " } else { "" };
                let member_name = &s.name;
                let displayed_type = display_type(&self.types(), s.ty);
                format!("{}{}: {}", pub_, member_name, displayed_type)
            }
            Method(s) => self.display_method_symbol(s),
            Trait(s) => {
                let s = symbols.get(s);
                format!("trait {} {}", s.name, display_raw_type(&self.types(), s.ty))
            }
            Variant(s) => {
                let s = symbols.get(s);
                let owner_name = symbols.get_symbol(s.owner.into()).name();
                // TODO: FIXME:
                format!("{}.{}", owner_name, s.name)
            }
        }
    }

    fn display_enum_symbol(&self, symbol: EnumSymbolId) -> String {
        let symbols = self.symbols.read().unwrap();
        let symbol = symbols.get(symbol);

        let name = &symbol.name;
        let ty = symbol.ty;
        let type_params = self.display_enum_type_params(ty);
        let variants = symbol
            .variants
            .iter()
            .map(|v| self.display_variant(*v))
            .map(|t| format!("    {t}"))
            .collect::<Vec<_>>();
        match variants.len() {
            0 => format!("enum {}{}{{}}", name, type_params),
            _ => format!(
                "enum {}{}{{\n{}\n}}",
                name,
                type_params,
                variants.join("\n")
            ),
        }
    }
    fn display_enum_type_params(&self, ty: TypeId) -> String {
        let store = self.types();
        let Type::Enum(e) = store.get(ty) else {
            panic!("expected function type")
        };
        if e.params.is_empty() {
            return String::new();
        }
        let params = e
            .params
            .iter()
            .map(|ty| ty.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        format!("<{}>", params)
    }
    fn display_variant(&self, symbol: VariantSymbolId) -> String {
        let symbols = self.symbols.read().unwrap();
        let variant = symbols.get(symbol);
        let args = variant
            .body
            .iter()
            .map(|a| display_type(&self.types(), symbols.get(*a).ty))
            .collect::<Vec<_>>();
        let name = variant.name.clone();
        if args.is_empty() {
            return name;
        }
        format!("{}({})", name, args.join(", "))
    }

    fn display_function_symbol(&self, symbol: FunctionSymbolId) -> String {
        let symbols = self.symbols.read().unwrap();
        let symbol = symbols.get(symbol);

        let name = &symbol.name;
        let ty = symbol.ty;

        let type_params = self.display_function_type_params(ty);
        let params = self.display_function_params(ty, &symbol.param_names);
        let return_type = match self.get_return_type(ty) {
            TypeStore::UNIT => String::new(),
            ty => {
                let displayed = display_type(&self.types.read().unwrap(), ty);
                format!(" -> {}", displayed)
            }
        };
        format!("fn {}{}({}){}", name, type_params, params, return_type)
    }

    fn display_method_symbol(&self, symbol: MethodSymbolId) -> String {
        let symbol = self.symbols().get(symbol).clone();
        let receiver = if symbol.receiver.is_static() {
            String::new()
        } else {
            let owner_name = self
                .symbols()
                .get_symbol(symbol.owner.into())
                .name()
                .to_string();
            let args = symbol
                .owner_args
                .iter()
                .map(|arg| display_type(&self.types(), *arg.1))
                .collect::<Vec<_>>()
                .join(", ");
            match args.len() {
                0 => format!("({}) ", owner_name),
                _ => format!("({}<{}>) ", owner_name, args),
            }
        };

        let params = self.display_function_params(symbol.ty, &symbol.param_names);
        let return_type = match self.get_return_type(symbol.ty) {
            TypeStore::UNIT => String::new(),
            t => format!(" -> {}", display_type(&self.types(), t)),
        };
        format!("fn {}{}({}){}", receiver, symbol.name, params, return_type)
    }

    fn display_function_params(&self, ty: TypeId, names: &Vec<String>) -> String {
        let store = self.types();
        let Type::Function(f) = store.get(ty) else {
            panic!("expected function type")
        };
        f.params
            .iter()
            .zip(names)
            .map(|(ty, name)| {
                let ty = display_type(&store, *ty);
                format!("{}: {}", name, ty)
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn display_function_type_params(&self, ty: TypeId) -> String {
        let store = self.types();
        let Type::Function(f) = store.get(ty) else {
            panic!("expected function type")
        };
        if f.type_params.is_empty() {
            return String::new();
        }
        format!(
            "<{}>",
            f.type_params
                .iter()
                .map(|ty| ty.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    /// Get the return type of a function or generic function
    fn get_return_type(&self, ty: TypeId) -> TypeId {
        let ty = self.types().get(ty).to_owned();
        match ty {
            Type::Function(FunctionType {
                ref return_type, ..
            }) => *return_type,
            _ => panic!(),
        }
    }
}
