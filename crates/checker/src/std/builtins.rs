use tine_symbols::symbols::*;
use tine_types::{store::TypeStore, types};

use crate::TypeChecker;

impl TypeChecker {
    pub(crate) fn init_builtins(&mut self) {
        self.int_builtin();
        self.float_builtin();
        self.eq_trait();
        self.hash_trait();
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

    fn eq_trait(&mut self) {
        let mut self_param = types::TypeParam {
            name: "S".into(),
            ..Default::default()
        };
        self_param.id = self.intern_unique(self_param.clone());

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
        let mut self_param = types::TypeParam {
            name: "S".into(),
            ..Default::default()
        };
        self_param.id = self.intern_unique(self_param.clone());

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
}
