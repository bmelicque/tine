use tine_ast::*;
use tine_common::locations::Location;

const HASH_INIT: i64 = 0x9747b28c;

pub fn derive_struct(node: &Option<TypeBody>, at: Location) -> ImplementationItem {
    let receiver = MethodReceiver {
        loc: at,
        mutable: false,
        pattern: Some(Identifier::new("self".to_string(), at).into()),
        self_type: None,
    };
    let stmts = match node {
        Some(TypeBody::Struct(s)) => derive_struct_struct(s, at),
        Some(TypeBody::Tuple(t)) => derive_tuple_struct(t, at),
        None => vec![IntLiteral::new(HASH_INIT, at).into()],
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
            params: vec![],
        }),
        return_type: Some(Type::Named(Identifier::new("bool".to_string(), at).into())),
        body: Some(BlockExpression {
            loc: at,
            statements: stmts,
        }),
    })
}

fn derive_struct_struct(node: &StructBody, at: Location) -> Vec<Statement> {
    let mut stmts = Vec::with_capacity(node.fields.len() + 2);
    stmts.push(init_hash(at).into());
    node.fields
        .iter()
        .filter_map(|field| field.name.as_ref())
        .map(|id| hash_combine_assignment(id.as_str(), at))
        .for_each(|stmt| stmts.push(stmt));
    stmts.push(Identifier::new("h".to_string(), at).into());
    stmts
}

fn derive_tuple_struct(node: &TupleBody, at: Location) -> Vec<Statement> {
    let len = node.elements.len();
    let mut stmts = Vec::with_capacity(len + 2);
    stmts.push(init_hash(at).into());
    (0..node.elements.len())
        .into_iter()
        .map(|field| format!("_{field}"))
        .map(|name| hash_combine_assignment(name.as_str(), at))
        .for_each(|stmt| stmts.push(stmt));
    stmts.push(Identifier::new("h".to_string(), at).into());
    stmts
}

fn hash_combine_assignment(field_name: &str, at: Location) -> Statement {
    let call = IntrinsicCall {
        loc: at,
        name: Identifier::new("hashCombine".into(), at),
        args: vec![
            Identifier::new("h".to_string(), at).into(),
            field_hash(field_name.to_owned(), at),
        ],
    };
    Statement::Assignment(Assignment {
        loc: at,
        pattern: Some(Assignee::Pattern(
            Identifier::new("h".to_string(), at).into(),
        )),
        value: Some(call.into()),
    })
}

fn init_hash(at: Location) -> VariableDeclaration {
    VariableDeclaration {
        loc: at,
        mutable: true,
        pattern: Some(Identifier::new("h".to_string(), at).into()),
        value: Some(IntLiteral::new(HASH_INIT, at).into()),
        ..Default::default()
    }
}

fn field_hash(name: String, loc: Location) -> Expression {
    let field = MemberExpression::valid(
        Identifier::new("self".to_string(), loc),
        Identifier::new(name.clone(), loc),
        loc,
    );
    Expression::Call(CallExpression {
        loc,
        callee: Some(Box::new(Expression::Member(MemberExpression::valid(
            field,
            Identifier::new("hash".to_string(), loc),
            loc,
        )))),
        args: vec![],
        type_args: None,
    })
}
