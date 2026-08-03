mod array;

use tine_symbols::symbols::*;
use tine_types::{store::TypeStore, types};

use crate::TypeChecker;

impl TypeChecker {
    pub(crate) fn init_builtins(&mut self) {
        self.primitive_builtin("int", TypeStore::INTEGER);
        self.primitive_builtin("float", TypeStore::FLOAT);
        self.primitive_builtin("bool", TypeStore::BOOLEAN);
        self.primitive_builtin("str", TypeStore::STRING);
        self.to_string_builtin(&["int", "float"]);
        self.array_builtin();
        self.map_builtin();
        self.eq_trait();
        self.hash_trait();
    }

    fn primitive_builtin(&mut self, name: &str, ty: types::TypeId) {
        let symbol: PrimitiveTypeSymbolId = self.symbols.insert(PrimitiveTypeSymbol {
            name: name.to_string(),
            ty,
            public: true,
            ..Default::default()
        });

        self.add_method(symbol, "eq", vec![ty], &["other"], TypeStore::BOOLEAN);
        self.add_method(symbol, "hash", vec![], &[], TypeStore::INTEGER);
    }

    fn to_string_builtin(&mut self, names: &[&str]) {
        for name in names {
            let symbol = self.builtin_id::<PrimitiveTypeSymbolId>(name).unwrap();
            self.add_method(symbol, "toString", vec![], &[], TypeStore::STRING);
        }
    }

    fn map_builtin(&mut self) {
        let key_param = self.make_type_param("K");
        let key_id = key_param.id;
        let value_param = self.make_type_param("V");
        let value_id = value_param.id;

        let map_type = self.intern_unique(types::StructType {
            params: vec![key_param, value_param],
            ..Default::default()
        });

        let map_symbol = self.symbols.insert::<StructSymbolId>(StructSymbol {
            name: "Map".into(),
            public: true,
            ty: map_type,
            ..Default::default()
        });

        let return_type = self.intern(types::OptionType { some: value_id });
        self.add_method(map_symbol, "get", vec![key_id], &["key"], return_type);

        self.add_method(
            map_symbol,
            "insert",
            vec![key_id, value_id],
            &["key", "value"],
            TypeStore::BOOLEAN,
        );

        self.add_method(
            map_symbol,
            "delete",
            vec![key_id],
            &["key"],
            TypeStore::BOOLEAN,
        );
    }

    fn eq_trait(&mut self) {
        let self_param = self.make_type_param("S");

        let eq_method_type = self.intern(types::FunctionType {
            type_params: vec![],
            params: vec![self_param.id],
            return_type: TypeStore::BOOLEAN,
        });

        let eq_trait_type = self.intern(types::TraitType {
            params: vec![],
            methods: vec![types::TraitMethod {
                self_type: Some(self_param.clone()),
                name: "eq".into(),
                def: eq_method_type,
            }],
        });

        self.symbols.insert::<TraitSymbolId>(TraitSymbol {
            name: "Eq".into(),
            public: true,
            ty: eq_trait_type,
            ..Default::default()
        });
    }

    fn hash_trait(&mut self) {
        let self_param = self.make_type_param("S");

        let hash_method_type = self.intern(types::FunctionType {
            return_type: TypeStore::INTEGER,
            ..Default::default()
        });

        let eq_trait_type = self.intern(types::TraitType {
            params: vec![],
            methods: vec![types::TraitMethod {
                self_type: Some(self_param.clone()),
                name: "hash".into(),
                def: hash_method_type,
            }],
        });

        self.symbols.insert::<TraitSymbolId>(TraitSymbol {
            name: "Hash".into(),
            public: true,
            ty: eq_trait_type,
            ..Default::default()
        });
    }

    fn make_type_param(&mut self, name: &str) -> types::TypeParam {
        let mut param = types::TypeParam {
            name: name.into(),
            ..Default::default()
        };
        param.id = self.intern_unique(param.clone());
        param
    }

    fn add_method<I>(
        &mut self,
        owner: I,
        name: &str,
        params: Vec<types::TypeId>,
        param_names: &[&str],
        return_type: types::TypeId,
    ) where
        I: Into<TypeSymbolId> + Copy,
    {
        self.add_method_helper(owner, name, params, param_names, return_type, false);
    }

    fn add_mutating_method<I>(
        &mut self,
        owner: I,
        name: &str,
        params: Vec<types::TypeId>,
        param_names: &[&str],
        return_type: types::TypeId,
    ) where
        I: Into<TypeSymbolId> + Copy,
    {
        self.add_method_helper(owner, name, params, param_names, return_type, true);
    }

    fn add_method_helper<I>(
        &mut self,
        owner: I,
        name: &str,
        params: Vec<types::TypeId>,
        param_names: &[&str],
        return_type: types::TypeId,
        mutating: bool,
    ) where
        I: Into<TypeSymbolId> + Copy,
    {
        let fn_type = self.intern(types::FunctionType {
            params,
            return_type,
            ..Default::default()
        });
        let receiver = if mutating {
            MethodReceiverKind::Mutable
        } else {
            MethodReceiverKind::Immutable
        };
        let fn_symbol: MethodSymbolId = self.symbols.insert(MethodSymbol {
            name: name.into(),
            public: true,
            owner: owner.into(),
            param_names: param_names.iter().map(|&s| s.into()).collect(),
            receiver,
            ty: fn_type,
            ..Default::default()
        });
        self.symbol_methods_mut(owner).push(fn_symbol);
    }
}
