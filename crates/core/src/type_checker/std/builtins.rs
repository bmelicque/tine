use crate::{
    symbols::*,
    type_store::TypeStore,
    types::{FunctionType, Type},
    TypeChecker,
};

impl TypeChecker {
    pub(crate) fn init_builtins(&mut self) {
        self.int_builtin();
        self.float_builtin();
    }

    fn int_builtin(&mut self) {
        let int_symbol: PrimitiveTypeSymbolId = self.symbols.insert(PrimitiveTypeSymbol {
            name: "int".to_string(),
            ty: TypeStore::INTEGER,
            ..Default::default()
        });

        let to_string_type = self.intern(Type::Function(FunctionType {
            return_type: TypeStore::STRING,
            ..Default::default()
        }));
        let to_string: MethodSymbolId = self.symbols.insert(MethodSymbol {
            name: "toString".into(),
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
            ty: TypeStore::FLOAT,
            ..Default::default()
        });

        let to_string_type = self.intern(Type::Function(FunctionType {
            return_type: TypeStore::STRING,
            ..Default::default()
        }));
        let to_string: MethodSymbolId = self.symbols.insert(MethodSymbol {
            name: "toString".into(),
            owner: float_symbol.into(),
            receiver: MethodReceiverKind::Immutable,
            ty: to_string_type,
            ..Default::default()
        });

        self.symbol_methods_mut(float_symbol).push(to_string);
    }
}
