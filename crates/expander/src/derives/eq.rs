use tine_ast::*;
use tine_common::locations::Location;

pub fn derive_struct(node: &Option<Vec<StructItem>>, at: Location) -> MethodDefinition {
    let default = BooleanLiteral::new(true, at).into();
    let expr = match node {
        Some(s) => s
            .iter()
            .filter_map(|f| f.as_field())
            .filter_map(|f| f.name.as_ref())
            .map(|name| field_eq(name.text.clone(), at))
            .reduce(|lhs, rhs| BinaryExpression::and(lhs, rhs, at).into())
            .unwrap_or(default),
        None => default,
    };
    MethodDefinition {
        loc: at,
        docs: None,
        public: true,
        static_: false,
        mut_: false,
        name: Some(Identifier::new("eq".to_string(), at)),
        type_params: None,
        params: Some(FunctionParams {
            loc: at,
            params: vec![other_param(at)],
        }),
        return_type: Some(Type::Named(Identifier::new("bool".to_string(), at).into())),
        body: Some(Box::new(Expression::Block(BlockExpression {
            loc: at,
            statements: vec![expr.into()],
        }))),
    }
}

/// `self.<name>.eq(other.<name>)`
fn field_eq(name: String, loc: Location) -> Expression {
    Expression::Call(CallExpression {
        loc,
        callee: Some(Box::new(Expression::Member(MemberExpression::valid(
            Expression::Member(MemberExpression {
                loc,
                object: None,
                prop: Some(Identifier::new(name.clone(), loc).into()),
            }),
            Identifier::new("eq".to_string(), loc),
            loc,
        )))),
        args: vec![Expression::Member(MemberExpression::valid(
            Identifier::new("other".to_string(), loc),
            Identifier::new(name, loc),
            loc,
        ))],
        type_args: None,
    })
}

fn other_param(at: Location) -> FunctionParam {
    FunctionParam {
        loc: at,
        name: Some(Identifier::new("other".to_string(), at)),
        type_annotation: Some(Type::Named(Identifier::new("Self".to_string(), at).into())),
    }
}
