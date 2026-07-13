use tine_common::locations::{Locatable, Location};

use crate::{
    nodes::{ast_enum, ast_struct},
    Identifier, MapType, TupleExpression, Type,
};

use super::{expressions::Expression, types::NamedType};

ast_struct!(ConstructorLiteral {
    qualifiers: Vec<Identifier>,
    constructor: Constructor,
    body: Option<ConstructorBody>,
});

ast_enum!(Constructor {
    Invalid(Type),

    Named(NamedType),
    Variant(VariantConstructor),
    Map(MapType),

});

ast_struct!(VariantConstructor {
    enum_name: Box<NamedType>,
    variant_name: Option<Identifier>,
});

ast_enum!(ConstructorBody {
    Struct(StructLiteralBody),
    Tuple(TupleExpression),
});

ast_struct!(StructLiteralBody {
    fields: Vec<ConstructorField>,
});

ast_struct!(ConstructorField {
    key: Option<ConstructorKey>,
    value: Option<Expression>,
});

ast_enum!(ConstructorKey {
    Name(Identifier),
    MapKey(Expression),
});
