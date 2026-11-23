use tine_ast::*;
use tine_common::locations::Location;

pub fn derive_struct(name: &str, body: &Option<Vec<StructItem>>, at: Location) -> MethodDefinition {
    let default = BooleanLiteral::new(true, at).into();
    let expr = body
        .as_ref()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|f| f.as_field())
        .filter_map(|f| f.name.as_ref())
        .map(|name| field_eq_method_call(name.text.clone(), at))
        .reduce(|lhs, rhs| BinaryExpression::and(lhs, rhs, at).into())
        .unwrap_or(default);
    method_definition(name, expr, at)
}

/// `.<name>.eq(other.<name>)`
fn field_eq_method_call(name: String, loc: Location) -> Expression {
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

pub fn derive_enum(name: &str, body: &Option<Vec<EnumItem>>, at: Location) -> MethodDefinition {
    let match_scrutinee = TupleExpression::from(vec![
        Expression::Member(MemberExpression::this(at)),
        Expression::Identifier(Identifier::new("other".to_string(), at)),
    ]);
    let expr = Expression::Match(MatchExpression {
        loc: at,
        scrutinee: Some(Box::new(match_scrutinee.into())),
        arms: derive_match_arms(body, at),
    });

    method_definition(name, expr, at)
}

fn derive_match_arms(body: &Option<Vec<EnumItem>>, loc: Location) -> Option<Vec<MatchArm>> {
    let mut arms = body
        .as_ref()?
        .into_iter()
        .filter_map(|i| derive_match_arm(i, loc))
        .collect::<Vec<_>>();
    arms.push(MatchArm {
        loc,
        pattern: Some(Box::new(Identifier::new("_".into(), loc).into())),
        expression: Some(Box::new(BooleanLiteral::new(false, loc).into())),
    });
    Some(arms)
}
fn derive_match_arm(item: &EnumItem, loc: Location) -> Option<MatchArm> {
    let variant = item.as_variant()?;
    let variant_name = variant.name.as_ref()?.as_str();
    match &variant.body {
        Some(body) => derive_match_arm_with_values(variant_name, body, loc).into(),
        None => derive_match_arm_with_units(variant_name, loc).into(),
    }
}
/// `(Variant, Variant) => true,`
fn derive_match_arm_with_units(name: &str, loc: Location) -> MatchArm {
    let unit_pattern: Pattern = Identifier::new(name.to_string(), loc).into();
    let pattern = TuplePattern {
        loc,
        elements: vec![unit_pattern.clone(), unit_pattern],
    };
    MatchArm {
        loc,
        pattern: Some(Box::new(pattern.into())),
        expression: Some(Box::new(BooleanLiteral::new(true, loc).into())),
    }
}
/// `(Variant(...a), Variant...(b)) => ...a == ...b`
fn derive_match_arm_with_values(name: &str, body: &VariantBody, loc: Location) -> MatchArm {
    let body_length = body.elements.len();

    let a = derive_match_arm_pattern(name, "a", body_length, loc);
    let b = derive_match_arm_pattern(name, "b", body_length, loc);
    let pattern = TuplePattern::from(vec![a, b]);

    let expression = (0..body_length)
        .map(|i| ai_eq_bi(i, loc))
        .reduce(|left, right| BinaryExpression::and(left, right, loc).into());

    MatchArm {
        loc,
        pattern: Some(Box::new(pattern.into())),
        expression: expression.map(Box::new),
    }
}
/// `Variant(...args)`
fn derive_match_arm_pattern(
    variant_name: &str,
    arg_prefix: &str,
    arg_count: usize,
    loc: Location,
) -> Pattern {
    let path = Identifier::new(variant_name.to_string(), loc).into();
    let args = (0..arg_count)
        .map(|i| Identifier::new(format!("{arg_prefix}{i}"), loc).into())
        .collect();
    Pattern::Call(CallPattern { loc, path, args })
}

/// `ai.eq(bi)`
fn ai_eq_bi(i: usize, loc: Location) -> Expression {
    Expression::Call(CallExpression {
        loc,
        callee: Some(Box::new(Expression::Member(MemberExpression::valid(
            Identifier::new(format!("a{i}"), loc),
            Identifier::new("eq".to_string(), loc),
            loc,
        )))),
        args: vec![Identifier::new(format!("b{i}"), loc).into()],
        type_args: None,
    })
}

fn method_definition(type_name: &str, expr: Expression, loc: Location) -> MethodDefinition {
    MethodDefinition {
        loc,
        docs: None,
        public: true,
        static_: false,
        mut_: false,
        name: Some(Identifier::new("eq".to_string(), loc)),
        type_params: None,
        params: Some(FunctionParams {
            loc,
            params: vec![other_param(type_name, loc)],
        }),
        return_type: Some(Identifier::new("bool".to_string(), loc).into()),
        body: Some(Box::new(expr)),
    }
}

fn other_param(name: &str, at: Location) -> FunctionParam {
    FunctionParam {
        loc: at,
        name: Some(Identifier::new("other".to_string(), at)),
        type_annotation: Some(Type::Named(Identifier::new(name.to_string(), at).into())),
    }
}
