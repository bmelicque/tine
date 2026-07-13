use tine_common::locations::{Locatable, Location};

use crate::{
    nodes::{ast_enum, ast_struct},
    Identifier,
};

ast_enum!(Type {
    Tuple(TupleType),
    Named(NamedType),
    Array(ArrayType),
    Function(FunctionType),
    Map(MapType),
    Option(OptionType),
    Result(ResultType),
});

ast_struct!(
    #[derive(Default)]
    NamedType {
        name: Identifier,
        args: Option<Vec<Type>>,
    }
);

ast_struct!(OptionType {
    base: Option<Box<Type>>,
});

ast_struct!(ArrayType {
    element: Option<Box<Type>>,
});

ast_struct!(TupleType {
    elements: Vec<Type>,
});

ast_struct!(MapType {
    key: Option<Box<Type>>,
    value: Option<Box<Type>>,
});

ast_struct!(ResultType {
    error: Option<Box<Type>>,
    ok: Option<Box<Type>>,
});

ast_struct!(FunctionType {
    params: Vec<Type>,
    returned: Option<Box<Type>>,
});
