mod alias;
mod liveness;
mod ownership;
mod semantics;

use tine_core::{ir, Session};

pub use ownership::{OwnershipAction, OwnershipMap};

pub fn analyse_program(program: &ir::Program, session: &Session) -> OwnershipMap {
    let sites = liveness::analyse_liveness(program);
    let mut aliases = alias::analyse_aliases(program, session);
    ownership::analyse_ownership(program, &sites, &mut aliases, session)
}
