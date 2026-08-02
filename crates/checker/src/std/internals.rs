use tine_symbols::symbols::*;
use tine_types::{store::TypeStore, types};

use crate::TypeChecker;

impl TypeChecker {
    pub fn init_internals(&mut self) {
        self.hash_internals();
    }

    fn hash_internals(&mut self) {
        self.add_hasher_symbol("hashBool", TypeStore::BOOLEAN);
        self.add_hasher_symbol("hashInt", TypeStore::INTEGER);
        self.add_hasher_symbol("hashFloat", TypeStore::FLOAT);
        self.add_hasher_symbol("hashString", TypeStore::STRING);
        self.add_hasher_symbol("hashAny", TypeStore::UNKNOWN);
        self.add_hasher_symbol("hashFinalize", TypeStore::INTEGER);
        self.add_hash_combine();
    }

    fn add_hasher_symbol(&mut self, name: &str, value_type: types::TypeId) {
        let ty = self.intern(types::FunctionType {
            type_params: vec![],
            params: vec![value_type],
            return_type: TypeStore::INTEGER,
        });
        self.symbols.insert::<FunctionSymbolId>(FunctionSymbol {
            name: name.to_string(),
            param_names: vec!["value".to_string()],
            ty,
            ..Default::default()
        });
    }

    fn add_hash_combine(&mut self) {
        let ty = self.intern(types::FunctionType {
            type_params: vec![],
            params: vec![TypeStore::INTEGER, TypeStore::INTEGER],
            return_type: TypeStore::INTEGER,
        });
        self.symbols.insert::<FunctionSymbolId>(FunctionSymbol {
            name: "hashCombine".to_string(),
            param_names: vec!["a".to_string(), "b".to_string()],
            ty,
            ..Default::default()
        });
    }
}
