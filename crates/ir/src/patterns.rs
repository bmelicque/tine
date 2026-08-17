use tine_common::locations::{Locatable, Location};
use tine_macros::tree_struct;
use tine_symbols::symbols::*;
use tine_types::store::TypeStore;

use crate::{
    ir_enum, BooleanLiteral, FloatLiteral, Identifier, IntLiteral, PushNodes, StringLiteral, Typed,
};

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
    #[child]
    pub arguments: Vec<Pattern>,
}

#[tree_struct]
#[derive(Debug, Clone)]
pub struct StructPattern {
    pub name: (Location, StructSymbolId),
    #[child]
    pub fields: Vec<StructPatternField>,
}

#[tree_struct(untyped)]
#[derive(Debug, Clone)]
pub struct StructPatternField {
    pub identifier: Identifier,
    #[child]
    pub pattern: Pattern,
}
impl<'a> PushNodes<'a> for StructPatternField {
    fn push_nodes(&'a self, stack: &mut Vec<crate::Node<'a>>) {
        self.pattern.push_nodes(stack);
    }
}

#[tree_struct]
#[derive(Debug, Clone)]
pub struct TuplePattern {
    #[child]
    pub elements: Vec<Pattern>,
}
