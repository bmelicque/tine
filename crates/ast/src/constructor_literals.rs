use tine_common::locations::{Locatable, Location};
use tine_macros::tree_struct;

use crate::{nodes::ast_enum, Identifier, MapType, TupleExpression, Type};

use super::{expressions::Expression, types::NamedType};

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ConstructorLiteral {
    pub qualifiers: Vec<Identifier>,
    pub constructor: Constructor,
    pub body: Option<ConstructorBody>,
}

ast_enum!(Constructor {
    Invalid(Type),

    Named(NamedType),
    Variant(VariantConstructor),
    Map(MapType),
});
impl Default for Constructor {
    fn default() -> Self {
        Self::Invalid(Type::Tuple(crate::TupleType::default()))
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct VariantConstructor {
    pub enum_name: Box<NamedType>,
    pub variant_name: Option<Identifier>,
}

ast_enum!(ConstructorBody {
    Struct(StructLiteralBody),
    Tuple(TupleExpression),
});

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct StructLiteralBody {
    pub fields: Vec<ConstructorField>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ConstructorField {
    pub key: Option<ConstructorKey>,
    pub value: Option<Expression>,
}

ast_enum!(ConstructorKey {
    Name(Identifier),
    MapKey(Expression),
});
