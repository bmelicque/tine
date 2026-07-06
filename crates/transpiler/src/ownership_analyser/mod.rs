mod alias;
mod liveness;
mod ownership;
mod semantics;

use tine_core::{ir, symbols::SymbolTable, type_store::TypeStore};

pub use ownership::{OwnershipAction, OwnershipMap};

pub fn analyse_program(
    program: &ir::Program,
    types: &TypeStore,
    symbols: &SymbolTable,
) -> OwnershipMap {
    let sites = liveness::analyse_liveness(program, symbols);
    let mut aliases = alias::analyse_aliases(program, types, symbols);
    ownership::analyse_ownership(program, &sites, &mut aliases, types, symbols)
}
