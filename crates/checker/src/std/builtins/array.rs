use tine_symbols::symbols::*;
use tine_types::{store::TypeStore, types::*};

use crate::TypeChecker;

impl TypeChecker {
    pub(super) fn array_builtin(&mut self) {
        let array_symbol = self.symbols.insert::<StructSymbolId>(StructSymbol {
            name: "Array".into(),
            public: true,
            ty: TypeStore::ARRAY,
            ..Default::default()
        });

        self.add_method(array_symbol, "length", vec![], &[], TypeStore::INTEGER);

        let return_type = self.option_type(TypeStore::ARRAY_PARAM);
        self.add_method(
            array_symbol,
            "get",
            vec![TypeStore::INTEGER],
            &["index"],
            return_type,
        );

        self.add_mutating_method(
            array_symbol,
            "set",
            vec![TypeStore::INTEGER, TypeStore::ARRAY_PARAM],
            &["index", "value"],
            TypeStore::BOOLEAN,
        );

        self.add_mutating_method(
            array_symbol,
            "push",
            vec![TypeStore::ARRAY_PARAM],
            &["element"],
            TypeStore::UNIT,
        );

        self.add_mutating_method(array_symbol, "pop", vec![], &[], return_type);

        self.array_map(array_symbol);
        self.array_filter(array_symbol);
    }

    fn array_map(&mut self, array_symbol: StructSymbolId) {
        let mut type_param = TypeParam {
            name: "U".into(),
            ..Default::default()
        };
        type_param.id = self.intern_unique(type_param.clone());
        let cb_type = self.intern(FunctionType {
            type_params: vec![],
            params: vec![TypeStore::ARRAY_PARAM],
            return_type: type_param.id,
        });
        let return_type = self.intern(TypeRef::new(TypeStore::ARRAY, vec![type_param.id]));
        let fn_type = self.intern(FunctionType {
            type_params: vec![type_param],
            params: vec![cb_type],
            return_type,
        });
        let map_method_symbol: MethodSymbolId = self.symbols.insert(MethodSymbol {
            name: "map".into(),
            public: true,
            owner: array_symbol.into(),
            param_names: vec!["f".into()],
            receiver: MethodReceiverKind::Immutable,
            ty: fn_type,
            ..Default::default()
        });
        self.symbol_methods_mut(array_symbol)
            .push(map_method_symbol);
    }

    fn array_filter(&mut self, array_symbol: StructSymbolId) {
        let predicate_type = self.intern(FunctionType {
            type_params: vec![],
            params: vec![TypeStore::ARRAY_PARAM],
            return_type: TypeStore::BOOLEAN,
        });
        let return_type = self.intern(TypeRef::new(TypeStore::ARRAY, vec![TypeStore::ARRAY_PARAM]));
        let fn_type = self.intern(FunctionType {
            type_params: vec![],
            params: vec![predicate_type],
            return_type,
        });
        let filter_method_symbol: MethodSymbolId = self.symbols.insert(MethodSymbol {
            name: "filter".into(),
            public: true,
            owner: array_symbol.into(),
            param_names: vec!["predicate".into()],
            receiver: MethodReceiverKind::Immutable,
            ty: fn_type,
            ..Default::default()
        });
        self.symbol_methods_mut(array_symbol)
            .push(filter_method_symbol);
    }
}
