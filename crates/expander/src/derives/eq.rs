use tine_ast::*;
use tine_common::locations::Location;

pub fn derive_struct(node: &Option<TypeBody>, at: Location) -> ImplementationItem {
    let receiver = MethodReceiver {
        loc: at,
        mutable: false,
        pattern: Some(Identifier::new("self".to_string(), at).into()),
        self_type: Some(Identifier::new("Self".to_string(), at)),
    };
    let param = FunctionParam {
        loc: at,
        name: Some(Identifier::new("other".to_string(), at)),
        type_annotation: Some(Type::Named(Identifier::new("Self".to_string(), at).into())),
    };
    let default = BooleanLiteral::new(true, at).into();
    let expr = match node {
        Some(TypeBody::Struct(s)) => s
            .fields
            .iter()
            .filter_map(|f| f.name.as_ref())
            .map(|name| field_eq(name.text.clone(), at))
            .reduce(|lhs, rhs| BinaryExpression::and(lhs, rhs, at).into())
            .unwrap_or(default),
        Some(TypeBody::Tuple(t)) => (0..t.elements.len())
            .into_iter()
            .map(|i| field_eq(format!("_{i}"), at))
            .reduce(|lhs, rhs| BinaryExpression::and(lhs, rhs, at).into())
            .unwrap_or(default),
        None => default,
    };

    ImplementationItem::Method(MethodDefinition {
        loc: at,
        docs: None,
        public: true,
        receiver,
        name: Some(Identifier::new("eq".to_string(), at)),
        type_params: None,
        params: Some(FunctionParams {
            loc: at,
            params: vec![param],
        }),
        return_type: Some(Type::Named(Identifier::new("bool".to_string(), at).into())),
        body: Some(BlockExpression {
            loc: at,
            statements: vec![expr.into()],
        }),
    })
}

fn field_eq(name: String, loc: Location) -> Expression {
    Expression::Call(CallExpression {
        loc,
        callee: Some(Box::new(Expression::Member(MemberExpression::valid(
            Expression::Member(MemberExpression::valid(
                Identifier::new("self".to_string(), loc),
                Identifier::new(name.clone(), loc),
                loc,
            )),
            Identifier::new("eq".to_string(), loc),
            loc,
        )))),
        args: vec![CallArgument::Expression(Expression::Member(
            MemberExpression::valid(
                Identifier::new("other".to_string(), loc),
                Identifier::new(name, loc),
                loc,
            ),
        ))],
        type_args: None,
    })
}
