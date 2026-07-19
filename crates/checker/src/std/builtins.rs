use tine_symbols::symbols::*;
use tine_types::{store::TypeStore, types};

use crate::TypeChecker;

impl TypeChecker {
    pub(crate) fn init_builtins(&mut self) {
        self.int_builtin();
        self.float_builtin();
    }

    fn int_builtin(&mut self) {
        let int_symbol: PrimitiveTypeSymbolId = self.symbols.insert(PrimitiveTypeSymbol {
            name: "int".to_string(),
            ty: TypeStore::INTEGER,
            public: true,
            ..Default::default()
        });

        let to_string_type = self.intern(types::FunctionType {
            return_type: TypeStore::STRING,
            ..Default::default()
        });
        let to_string: MethodSymbolId = self.symbols.insert(MethodSymbol {
            name: "toString".into(),
            public: true,
            owner: int_symbol.into(),
            receiver: MethodReceiverKind::Immutable,
            ty: to_string_type,
            ..Default::default()
        });

        self.symbol_methods_mut(int_symbol).push(to_string);
    }

    fn float_builtin(&mut self) {
        let float_symbol: PrimitiveTypeSymbolId = self.symbols.insert(PrimitiveTypeSymbol {
            name: "float".to_string(),
            public: true,
            ty: TypeStore::FLOAT,
            ..Default::default()
        });

        let to_string_type = self.intern(types::FunctionType {
            return_type: TypeStore::STRING,
            ..Default::default()
        });
        let to_string: MethodSymbolId = self.symbols.insert(MethodSymbol {
            name: "toString".into(),
            public: true,
            owner: float_symbol.into(),
            receiver: MethodReceiverKind::Immutable,
            ty: to_string_type,
            ..Default::default()
        });

        self.symbol_methods_mut(float_symbol).push(to_string);
    }
}
