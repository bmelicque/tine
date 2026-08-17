use tine_symbols::{symbols::*, table::SymbolTable};
use tine_types::{store::TypeStore, types};

pub struct SemanticsChecker<'ty, 'sym> {
    types: &'ty TypeStore,
    symbols: &'sym SymbolTable,
}

impl SemanticsChecker<'_, '_> {
    pub fn new<'ty, 'sym>(
        types: &'ty TypeStore,
        symbols: &'sym SymbolTable,
    ) -> SemanticsChecker<'ty, 'sym> {
        SemanticsChecker { types, symbols }
    }

    /// Check if the given type implements Copy semantics.
    pub fn is_copy(&self, ty: types::TypeId) -> bool {
        use types::Type::*;
        let ty = self.types.get(ty);
        match ty {
            Boolean | Float | Integer | String | Signal(_) | Listener(_) => true,
            _ => false,
        }
    }

    pub fn is_trait(&self, ty: types::TypeId) -> bool {
        let ty = self.types.get(ty);
        matches!(ty, types::Type::Trait(_))
    }

    pub fn symbols(&self) -> &SymbolTable {
        &self.symbols
    }

    pub fn get_symbol(&self, id: SymbolId) -> &dyn Symbol {
        self.symbols.get_symbol(id)
    }

    pub fn is_mutable_symbol(&self, id: SymbolId) -> bool {
        self.symbols.is_mutable(id)
    }

    pub fn is_immutable_symbol(&self, id: SymbolId) -> bool {
        !self.symbols.is_mutable(id)
    }
}
