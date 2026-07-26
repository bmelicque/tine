use tine_ast::*;
use tine_common::locations::Location;

pub fn derive_struct(node: &Option<TypeBody>, at: Location) -> ImplementationItem {
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
        receiver: eq_receiver(at),
        name: Some(Identifier::new("eq".to_string(), at)),
        type_params: None,
        params: Some(FunctionParams {
            loc: at,
            params: vec![other_param(at)],
        }),
        return_type: Some(Type::Named(Identifier::new("bool".to_string(), at).into())),
        body: Some(BlockExpression {
            loc: at,
            statements: vec![expr.into()],
        }),
    })
}

pub fn derive_enum(node: &EnumDefinition, at: Location) -> ImplementationItem {
    let mut arms: Vec<MatchArm> = node
        .variants
        .iter()
        .filter_map(|v| v.name.as_ref().map(|name| variant_arm(v, name, at)))
        .collect();
    arms.push(wildcard_arm(at));

    let scrutinee = TupleExpression {
        elements: vec![
            Identifier::new("self".to_string(), at).into(),
            Identifier::new("other".to_string(), at).into(),
        ],
        ..Default::default()
    };

    let match_expr = MatchExpression {
        scrutinee: Some(Box::new(scrutinee.into())),
        arms: Some(arms),
        ..Default::default()
    };

    ImplementationItem::Method(MethodDefinition {
        loc: at,
        docs: None,
        public: true,
        receiver: eq_receiver(at),
        name: Some(Identifier::new("eq".to_string(), at)),
        type_params: None,
        params: Some(FunctionParams {
            loc: at,
            params: vec![other_param(at)],
        }),
        return_type: Some(Type::Named(Identifier::new("bool".to_string(), at).into())),
        body: Some(BlockExpression {
            loc: at,
            statements: vec![Expression::Match(match_expr).into()],
        }),
    })
}

/// One `(Self::Variant(bindings...), Self::Variant(bindings...)) => ...` arm.
fn variant_arm(variant: &VariantDefinition, name: &Identifier, at: Location) -> MatchArm {
    let keys = variant_field_keys(&variant.body);

    let pattern = TuplePattern {
        loc: at,
        elements: vec![
            variant_constructor_pattern(&variant.body, &keys, name, "self", at),
            variant_constructor_pattern(&variant.body, &keys, name, "other", at),
        ],
    };

    MatchArm {
        pattern: Some(Box::new(pattern.into())),
        expression: Some(Box::new(variant_eq_expr(&keys, at))),
        ..Default::default()
    }
}

/// Final `_ => false` arm for mismatched variants.
fn wildcard_arm(at: Location) -> MatchArm {
    MatchArm {
        pattern: Some(Box::new(Identifier::new("_".to_string(), at).into())),
        expression: Some(Box::new(BooleanLiteral::new(false, at).into())),
        ..Default::default()
    }
}

/// Field keys for a variant body: struct-variant field names, or
/// stringified indices for a tuple variant. Empty for a unit variant.
fn variant_field_keys(body: &Option<TypeBody>) -> Vec<String> {
    match body {
        Some(TypeBody::Struct(s)) => s
            .fields
            .iter()
            .filter_map(|f| f.name.as_ref())
            .map(|n| n.text.clone())
            .collect(),
        Some(TypeBody::Tuple(t)) => (0..t.elements.len()).map(|i| i.to_string()).collect(),
        None => vec![],
    }
}

/// `Variant`, `Variant(_self_0, _self_1)` or `Variant { x: _self_x, .. }`
fn variant_constructor_pattern(
    body: &Option<TypeBody>,
    keys: &[String],
    variant_name: &Identifier,
    side: &str,
    at: Location,
) -> Pattern {
    ConstructorPattern {
        loc: at,
        qualifiers: vec![],
        constructor: Constructor::Named(NamedType {
            loc: at,
            name: variant_name.clone(),
            args: None,
        }),
        body: variant_pattern_body(body, keys, side, at),
    }
    .into()
}

fn variant_pattern_body(
    body: &Option<TypeBody>,
    keys: &[String],
    side: &str,
    at: Location,
) -> Option<ConstructorPatternBody> {
    match body {
        Some(TypeBody::Struct(s)) => Some(
            StructPatternBody {
                loc: at,
                fields: s
                    .fields
                    .iter()
                    .filter_map(|f| f.name.as_ref())
                    .map(|field_name| StructPatternField {
                        loc: at,
                        identifier: Some(FieldPatternIdentifier::Const(field_name.clone())),
                        pattern: Some(
                            Identifier::new(bound_name(side, &field_name.text), at).into(),
                        ),
                    })
                    .collect(),
            }
            .into(),
        ),
        Some(TypeBody::Tuple(_)) => Some(
            TuplePattern {
                loc: at,
                elements: keys
                    .iter()
                    .map(|k| Identifier::new(bound_name(side, k), at).into())
                    .collect(),
            }
            .into(),
        ),
        None => None,
    }
}

fn bound_name(side: &str, key: &str) -> String {
    format!("_{side}_{key}")
}

/// Conjunction of `_self_k.eq(_other_k)` across all bound fields, or `true`
/// for a unit variant.
fn variant_eq_expr(keys: &[String], at: Location) -> Expression {
    let default = BooleanLiteral::new(true, at).into();
    keys.iter()
        .map(|k| bound_eq(bound_name("self", k), bound_name("other", k), at))
        .reduce(|lhs, rhs| BinaryExpression::and(lhs, rhs, at).into())
        .unwrap_or(default)
}

/// `<self_name>.eq(<other_name>)` between two locally-bound identifiers
fn bound_eq(self_name: String, other_name: String, loc: Location) -> Expression {
    Expression::Call(CallExpression {
        loc,
        callee: Some(Box::new(Expression::Member(MemberExpression::valid(
            Identifier::new(self_name, loc),
            Identifier::new("eq".to_string(), loc),
            loc,
        )))),
        args: vec![CallArgument::Expression(
            Identifier::new(other_name, loc).into(),
        )],
        type_args: None,
    })
}

/// `self.<name>.eq(other.<name>)`
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

fn eq_receiver(at: Location) -> MethodReceiver {
    MethodReceiver {
        loc: at,
        mutable: false,
        pattern: Some(Identifier::new("self".to_string(), at).into()),
        self_type: Some(Identifier::new("Self".to_string(), at)),
    }
}

fn other_param(at: Location) -> FunctionParam {
    FunctionParam {
        loc: at,
        name: Some(Identifier::new("other".to_string(), at)),
        type_annotation: Some(Type::Named(Identifier::new("Self".to_string(), at).into())),
    }
}
