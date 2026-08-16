use tine_common::locations::{Locatable, Location};
use tine_macros::tree_struct;
use tine_symbols::symbols::*;
use tine_types::store::TypeStore;

use crate::{ir_enum, BooleanLiteral, FloatLiteral, Identifier, IntLiteral, StringLiteral, Typed};

ir_enum!(Pattern {
    Boolean(BooleanLiteral),
    Float(FloatLiteral),
    Integer(IntLiteral),
    String(StringLiteral),

    Call(CallPattern),
    Identifier(Identifier),
    Struct(StructPattern),
    Tuple(TuplePattern),
});
impl Pattern {
    pub fn wildcard() -> Self {
        Self::Identifier(Identifier {
            loc: Location::dummy(),
            ty: TypeStore::UNKNOWN,
            symbol: SymbolId::dummy(),
        })
    }
}

#[tree_struct]
#[derive(Debug, Clone)]
pub struct CallPattern {
    pub callee: (Location, VariantSymbolId),
    pub arguments: Vec<Pattern>,
}

#[tree_struct]
#[derive(Debug, Clone)]
pub struct StructPattern {
    pub name: (Location, StructSymbolId),
    pub fields: Vec<StructPatternField>,
}

#[derive(Debug, Clone)]
pub struct StructPatternField {
    pub loc: Location,
    pub identifier: Identifier,
    pub pattern: Option<Pattern>,
}

#[tree_struct]
#[derive(Debug, Clone)]
pub struct TuplePattern {
    pub elements: Vec<Pattern>,
}
