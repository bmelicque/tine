use crate::{
    type_checker::SymbolHandle,
    types::{FunctionType, Type},
    Session, SymbolData, SymbolKind, SymbolRef, TypeStore,
};

impl Session {
    pub(super) fn init_builtins(&mut self) {
        self.int_builtin();
        self.float_builtin();
    }

    fn int_builtin(&mut self) {
        let int_handle = self.add_builtin(SymbolData {
            name: "int".into(),
            ty: TypeStore::INTEGER,
            kind: SymbolKind::PrimitiveType { methods: vec![] },
            // TODO: add docs
            ..Default::default()
        });
        let int_to_string_handle = self.add_builtin(SymbolData {
            name: "toString".into(),
            ty: self.intern(Type::Function(FunctionType {
                params: vec![],
                return_type: TypeStore::STRING,
            })),
            kind: SymbolKind::Method {
                owner: int_handle.readonly(),
                owner_args: vec![],
                has_receiver: true,
                param_names: vec![],
            },
            ..Default::default()
        });
        match int_handle.borrow().kind {
            SymbolKind::PrimitiveType { ref mut methods } => {
                methods.push(int_to_string_handle.readonly());
            }
            ref kind => unreachable!("expected primitive, got {:?}", kind),
        };
    }

    fn float_builtin(&mut self) {
        let float_handle = self.add_builtin(SymbolData {
            name: "float".into(),
            ty: TypeStore::FLOAT,
            kind: SymbolKind::PrimitiveType { methods: vec![] },
            // TODO: add docs
            ..Default::default()
        });
        let string_handle = self.add_builtin(SymbolData {
            name: "toString".into(),
            ty: self.intern(Type::Function(FunctionType {
                params: vec![],
                return_type: TypeStore::STRING,
            })),
            kind: SymbolKind::Method {
                owner: float_handle.readonly(),
                owner_args: vec![],
                has_receiver: true,
                param_names: vec![],
            },
            ..Default::default()
        });
        match float_handle.borrow().kind {
            SymbolKind::PrimitiveType { ref mut methods } => {
                methods.push(string_handle.readonly());
            }
            _ => unreachable!(),
        };
    }

    fn add_builtin(&mut self, symbol: SymbolData) -> SymbolHandle {
        let handle = SymbolHandle::new(symbol);
        self.builtins.push(handle.readonly());
        self.symbols.push(handle.clone());
        handle
    }

    pub fn find_builtin(&self, name: &str) -> Option<SymbolRef> {
        self.builtins
            .iter()
            .find(|s| s.borrow().name == name)
            .cloned()
    }
}
