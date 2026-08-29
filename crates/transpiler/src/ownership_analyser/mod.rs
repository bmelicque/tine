mod alias;
mod liveness;
mod ownership;
mod semantics;

pub use ownership::{OwnershipAction, OwnershipMap};
use tine_ir::Program;
use tine_symbols::table::SymbolTable;
use tine_types::store::TypeStore;

pub fn analyse_program(
    program: &Program,
    types: &TypeStore,
    symbols: &SymbolTable,
) -> OwnershipMap {
    let sites = liveness::analyse_liveness(program, symbols);
    let mut aliases = alias::analyse_aliases(program, types, symbols);
    ownership::analyse_ownership(program, &sites, &mut aliases, types, symbols)
}
