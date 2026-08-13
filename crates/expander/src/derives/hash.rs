use tine_ast::*;
use tine_common::locations::Location;

const HASH_INIT: i64 = 0x9747b28c;

pub fn derive_struct(node: &Option<Vec<StructItem>>, at: Location) -> MethodDefinition {
    let stmts = match node {
        Some(s) => derive_struct_body(s, at),
        None => vec![IntLiteral::new(HASH_INIT, at).into()],
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
            params: vec![],
        }),
        return_type: Some(Type::Named(Identifier::new("bool".to_string(), at).into())),
        body: Some(Box::new(Expression::Block(BlockExpression {
            loc: at,
            statements: stmts,
        }))),
    }
}

fn derive_struct_body(body: &Vec<StructItem>, at: Location) -> Vec<Statement> {
    let mut stmts = Vec::new();
    stmts.push(init_hash(at).into());
    body.into_iter()
        .filter_map(|f| f.as_field())
        .filter_map(|field| field.name.as_ref())
        .map(|id| hash_combine_assignment(id.as_str(), at))
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
