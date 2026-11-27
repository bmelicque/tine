use tine_symbols::symbols::*;
use tine_types::{store::TypeStore, types::TypeRef};

use crate::TypeChecker;

impl TypeChecker {
    /// Needs `Map` builtin
    pub(super) fn json_builtin(&mut self) {
        let ty = self.types.well_known("Json");
        let json_symbol = self.insert::<StructSymbolId>(StructSymbol {
            name: "Json".into(),
            public: true,
            ty,
            ..Default::default()
        });
        let parse_return = self.result_type(ty, TypeStore::STRING);
        self.add_static_method(
            json_symbol,
            "parse",
            vec![TypeStore::STRING],
            &["src"],
            parse_return,
        );

        let bool_option = self.option_type(TypeStore::BOOLEAN);
        self.add_method(json_symbol, "asBool", vec![], &[], bool_option);

        let int_option = self.option_type(TypeStore::INTEGER);
        self.add_method(json_symbol, "asInt", vec![], &[], int_option);

        let float_option = self.option_type(TypeStore::FLOAT);
        self.add_method(json_symbol, "asFloat", vec![], &[], float_option);

        let str_option = self.option_type(TypeStore::STRING);
        self.add_method(json_symbol, "asStr", vec![], &[], str_option);

        let json_array = self.intern(TypeRef::new(TypeStore::ARRAY, vec![ty]));
        let array_option = self.option_type(json_array);
        self.add_method(json_symbol, "asArray", vec![], &[], array_option);

        let object_ty = self.types.well_known("JsonObject");
        let object_option = self.option_type(object_ty);
        self.add_method(json_symbol, "asObject", vec![], &[], object_option);

        let json_object_symbol = self.symbols.insert::<StructSymbolId>(StructSymbol {
            name: "JsonObject".into(),
            public: true,
            ty: object_ty,
            ..Default::default()
        });
        let json_option = self.option_type(ty);
        self.add_method(
            json_object_symbol,
            "get",
            vec![TypeStore::STRING],
            &["key"],
            json_option,
        );
    }
}
