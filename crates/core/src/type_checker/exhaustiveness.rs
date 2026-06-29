use std::{collections::HashSet, sync::LazyLock};

use crate::{
    ir,
    type_checker::patterns::{
        ConstructorPattern, LiteralPattern, Pattern, PatternField, StructPattern, TuplePattern,
    },
    Location, SymbolRef,
};

static DEFAULT_STRUCT: LazyLock<StructPattern> = LazyLock::new(StructPattern::default);
static DEFAULT_TUPLE: LazyLock<TuplePattern> = LazyLock::new(TuplePattern::default);

type Row<'p> = Vec<&'p Pattern>;
type Matrix<'p> = Vec<Row<'p>>;

struct Specialization<'p> {
    new_arms: Matrix<'p>,
    new_query: Row<'p>,
}

fn specialize_literal<'p>(
    arms: &Matrix<'p>,
    query_head: &'p LiteralPattern,
    query_tail: &[&'p Pattern],
) -> Specialization<'p> {
    let literal_arms = arms.into_iter().filter_map(|arm| {
        let (head, tail) = arm.split_first()?;
        head.as_literal().map(|lit| (lit, tail))
    });
    let new_arms = match query_head {
        LiteralPattern::Bool(q) => literal_arms
            .filter_map(|(head, tail)| match head.as_bool() {
                Some(b) if b.value == q.value => Some(tail.to_vec()),
                _ => None,
            })
            .collect(),
        LiteralPattern::Float(q) => literal_arms
            .filter_map(|(head, tail)| match head.as_float() {
                Some(f) if f.value == q.value => Some(tail.to_vec()),
                _ => None,
            })
            .collect(),
        LiteralPattern::Int(q) => literal_arms
            .filter_map(|(head, tail)| match head.as_int() {
                Some(i) if i.value == q.value => Some(tail.to_vec()),
                _ => None,
            })
            .collect(),
        LiteralPattern::String(q) => literal_arms
            .filter_map(|(head, tail)| match head.as_string() {
                Some(s) if s.value == q.value => Some(tail.to_vec()),
                _ => None,
            })
            .collect(),
    };

    Specialization {
        new_arms,
        new_query: query_tail.to_vec(),
    }
}

fn specialize_constructor<'p>(
    arms: &Matrix<'p>,
    query_head: &'p ConstructorPattern,
    query_tail: &[&'p Pattern],
) -> Specialization<'p> {
    let arity = query_head.identifier.symbol.arity();
    let new_arms = arms
        .into_iter()
        .filter_map(|arm| {
            let (head, tail) = arm.split_first()?;
            head.as_constructor().map(|lit| (lit, tail))
        })
        .filter(|(head, _)| head.identifier.symbol == query_head.identifier.symbol)
        .map(|(head, tail)| {
            if arity > 0 {
                let mut tail = tail.to_vec();
                tail.insert(0, head.arg.as_deref().unwrap_or(&Pattern::Wildcard));
                tail
            } else {
                tail.to_vec()
            }
        })
        .collect();

    let new_query = if arity > 0 {
        let mut tail = query_tail.to_vec();
        tail.insert(0, query_head.arg.as_deref().unwrap_or(&Pattern::Wildcard));
        tail
    } else {
        query_tail.to_vec()
    };

    Specialization {
        new_arms,
        new_query,
    }
}

fn specialize_struct<'p>(
    arms: &'p Matrix<'p>,
    query_head: &'p StructPattern,
    query_tail: &'p [&'p Pattern],
) -> (Specialization<'p>, Vec<SymbolRef>) {
    let structs = get_structs(arms);
    let all_keys = all_keys_from_structs(&structs, Some(query_head));

    let new_arms = structs
        .into_iter()
        .map(|(st, tail)| unwrap_fields(&st.fields, tail, &all_keys))
        .collect();
    let new_query = unwrap_fields(&query_head.fields, query_tail, &all_keys);

    let s = Specialization {
        new_arms,
        new_query,
    };
    (s, all_keys)
}
fn specialize_tuple<'p>(
    arms: &'p Matrix<'p>,
    query_head: &'p TuplePattern,
    query_tail: &'p [&'p Pattern],
) -> (Specialization<'p>, Vec<SymbolRef>) {
    let tuples = get_tuples(arms);
    let all_keys = all_keys_from_tuples(&tuples, Some(query_head));

    let new_arms = tuples
        .into_iter()
        .map(|(t, tail)| unwrap_fields(&t.items, tail, &all_keys))
        .collect();
    let new_query = unwrap_fields(&query_head.items, query_tail, &all_keys);

    let s = Specialization {
        new_arms,
        new_query,
    };
    (s, all_keys)
}
fn unwrap_fields<'p>(
    fields: &'p [PatternField],
    tail: &'p [&Pattern],
    sorted_keys: &[SymbolRef],
) -> Vec<&'p Pattern> {
    let mut arm: Vec<&Pattern> = sorted_keys
        .iter()
        .map(|k| {
            fields
                .iter()
                .find(|f| f.0.symbol == *k)
                .map_or(&Pattern::Wildcard, |f| &f.1)
        })
        .collect::<Vec<_>>();
    arm.extend_from_slice(tail);
    arm
}

/// Returns a list of patterns covered by the query that were not covered by
/// any arm.
pub fn usefulness(arms: &Matrix, query: &[&Pattern]) -> Vec<Vec<Pattern>> {
    let (head, tail) = query.split_first().unwrap();
    match head {
        Pattern::Literal(p) => literal_usefulness(arms, p, tail),
        Pattern::Constructor(p) => constructor_usefulness(arms, p, tail),
        Pattern::Struct(p) => struct_usefulness(arms, p, tail),
        Pattern::Tuple(p) => tuple_usefulness(arms, p, tail),
        Pattern::Identifier(_) | Pattern::Wildcard => wildcard_usefulness(arms, tail),
    }
}

fn literal_usefulness(
    arms: &Matrix,
    query_head: &LiteralPattern,
    query_tail: &[&Pattern],
) -> Vec<Vec<Pattern>> {
    let got = specialize_literal(arms, query_head, query_tail);
    if usefulness(&got.new_arms, &got.new_query).is_empty() {
        vec![vec![Pattern::Wildcard]]
    } else {
        vec![]
    }
}

fn constructor_usefulness(
    arms: &Matrix,
    query_head: &ConstructorPattern,
    query_tail: &[&Pattern],
) -> Vec<Vec<Pattern>> {
    let arity = query_head.identifier.symbol.arity();
    let s = specialize_constructor(arms, query_head, query_tail);
    usefulness(&s.new_arms, &s.new_query)
        .into_iter()
        .map(|mut witness| {
            let (arg, mut tail) = if arity > 0 {
                let first = witness.remove(0);
                (Some(Box::new(first)), witness)
            } else {
                (None, witness)
            };
            let pat = Pattern::Constructor(ConstructorPattern {
                identifier: query_head.identifier.clone(),
                arg,
            });
            tail.insert(0, pat);
            tail
        })
        .collect()
}

fn struct_usefulness(
    arms: &Matrix,
    query_head: &StructPattern,
    query_tail: &[&Pattern],
) -> Vec<Vec<Pattern>> {
    let (s, keys) = specialize_struct(arms, query_head, query_tail);
    usefulness(&s.new_arms, &s.new_query)
        .into_iter()
        .map(|witness| {
            let (fields, mut tail) = reconstruct_fields(witness, &keys);
            let pat = StructPattern { fields };
            tail.insert(0, pat.into());
            tail
        })
        .collect()
}
fn tuple_usefulness(
    arms: &Matrix,
    query_head: &TuplePattern,
    query_tail: &[&Pattern],
) -> Vec<Vec<Pattern>> {
    let (s, keys) = specialize_tuple(arms, query_head, query_tail);
    usefulness(&s.new_arms, &s.new_query)
        .into_iter()
        .map(|witness| {
            let (items, mut tail) = reconstruct_fields(witness, &keys);
            let pat = TuplePattern { items };
            tail.insert(0, pat.into());
            tail
        })
        .collect()
}
/// Returns `(fields, witness_tail)`
fn reconstruct_fields(
    mut witness: Vec<Pattern>,
    keys: &[SymbolRef],
) -> (Vec<PatternField>, Vec<Pattern>) {
    let tail = witness.split_off(keys.len());
    let fields = keys
        .iter()
        .zip(witness)
        .map(|(key, pattern)| {
            PatternField(
                ir::Identifier {
                    loc: Location::dummy(),
                    symbol: key.clone(),
                },
                pattern,
            )
        })
        .collect();
    (fields, tail)
}

fn wildcard_usefulness(arms: &Matrix, query_tail: &[&Pattern]) -> Vec<Vec<Pattern>> {
    // TODO: handle tuples and structs
    let first = arms
        .iter()
        .map(|arm| arm[0])
        .filter(|pat| !is_wildcard(pat))
        .next();
    let Some(first) = first else {
        let arms = arms
            .into_iter()
            .map(|arm| arm[1..].to_vec())
            .collect::<Vec<_>>();

        return usefulness(&arms, query_tail);
    };

    match first {
        Pattern::Identifier(_) | Pattern::Wildcard => unreachable!(),

        Pattern::Literal(_) => wildcard_usefulness_literal(arms, query_tail),
        Pattern::Constructor(c) => wildcard_usefulness_constructor(arms, c, query_tail),
        Pattern::Struct(_) => wildcard_usefulness_struct(arms, query_tail),
        Pattern::Tuple(_) => wildcard_usefulness_tuple(arms, query_tail),
    }
}
fn wildcard_usefulness_literal(arms: &Matrix, query_tail: &[&Pattern]) -> Vec<Vec<Pattern>> {
    let bools = arms
        .into_iter()
        .filter_map(|arm| arm.split_first())
        .filter_map(|(head, tail)| Some((head.as_literal()?, tail)))
        .filter_map(|(head, tail)| Some((head.as_bool()?, tail)))
        .collect::<Vec<_>>();
    if bools.is_empty() {
        let arms = arms
            .into_iter()
            .map(|arm| arm[1..].to_vec())
            .collect::<Vec<_>>();
        return usefulness(&arms, query_tail);
    }
    let true_pat = LiteralPattern::Bool(ir::BooleanLiteral {
        loc: Location::dummy(),
        value: true,
    });
    let false_pat = LiteralPattern::Bool(ir::BooleanLiteral {
        loc: Location::dummy(),
        value: false,
    });
    vec![true_pat, false_pat]
        .into_iter()
        .flat_map(|pat| literal_usefulness(arms, &pat.into(), query_tail))
        .collect()
}
fn wildcard_usefulness_constructor(
    arms: &Matrix,
    example: &ConstructorPattern,
    query_tail: &[&Pattern],
) -> Vec<Vec<Pattern>> {
    let variants = example
        .identifier
        .symbol
        .borrow()
        .owner()
        .and_then(|o| o.as_variants())
        .unwrap_or(vec![]);

    variants
        .into_iter()
        .flat_map(|symbol| {
            let arg = if symbol.arity() > 0 {
                Some(Box::new(Pattern::Wildcard))
            } else {
                None
            };
            let pattern = ConstructorPattern {
                identifier: ir::Identifier::locless(symbol),
                arg,
            };
            constructor_usefulness(arms, &pattern, query_tail)
        })
        .collect()
}
fn wildcard_usefulness_struct(arms: &Matrix, query_tail: &[&Pattern]) -> Vec<Vec<Pattern>> {
    let query = get_struct_signature(arms);
    struct_usefulness(arms, &query, query_tail)
}

fn get_struct_signature(arms: &Matrix) -> StructPattern {
    let structs = get_structs(arms);
    let all_keys = all_keys_from_structs(&structs, None);
    let fields = all_keys
        .into_iter()
        .map(|symbol| {
            PatternField(
                ir::Identifier {
                    loc: Location::dummy(),
                    symbol,
                },
                Pattern::Wildcard,
            )
        })
        .collect();
    StructPattern { fields }
}

fn get_structs<'p>(arms: &'p Matrix) -> Vec<(&'p StructPattern, &'p [&'p Pattern])> {
    arms.into_iter()
        .filter_map(|arm| {
            let (head, tail) = arm.split_first()?;
            let head = match head {
                Pattern::Struct(st) => st,
                p if is_wildcard(p) => &DEFAULT_STRUCT,
                _ => return None,
            };
            Some((head, tail))
        })
        .collect()
}
fn all_keys_from_structs(
    structs: &[(&StructPattern, &[&Pattern])],
    additional: Option<&StructPattern>,
) -> Vec<SymbolRef> {
    let all_keys = structs
        .iter()
        .map(|(head, _)| head)
        .cloned()
        .chain(additional)
        .flat_map(|pat| &pat.fields)
        .map(|field| field.0.symbol.clone())
        .collect::<HashSet<_>>();
    let mut all_keys = all_keys.into_iter().collect::<Vec<_>>();
    all_keys.sort_by_key(|k| k.as_name());
    all_keys
}

fn wildcard_usefulness_tuple(arms: &Matrix, query_tail: &[&Pattern]) -> Vec<Vec<Pattern>> {
    let query = get_tuple_signature(arms);
    tuple_usefulness(arms, &query, query_tail)
}

fn get_tuple_signature(arms: &Matrix) -> TuplePattern {
    let tuples = get_tuples(arms);
    let all_keys = all_keys_from_tuples(&tuples, None);
    let items = all_keys
        .into_iter()
        .map(|symbol| {
            PatternField(
                ir::Identifier {
                    loc: Location::dummy(),
                    symbol,
                },
                Pattern::Wildcard,
            )
        })
        .collect();
    TuplePattern { items }
}

fn get_tuples<'p>(arms: &'p Matrix) -> Vec<(&'p TuplePattern, &'p [&'p Pattern])> {
    arms.into_iter()
        .filter_map(|arm| {
            let (head, tail) = arm.split_first()?;
            let head = match head {
                Pattern::Tuple(st) => st,
                p if is_wildcard(p) => &DEFAULT_TUPLE,
                _ => return None,
            };
            Some((head, tail))
        })
        .collect()
}
fn all_keys_from_tuples(
    tuples: &[(&TuplePattern, &[&Pattern])],
    additional: Option<&TuplePattern>,
) -> Vec<SymbolRef> {
    let all_keys = tuples
        .iter()
        .map(|(head, _)| head)
        .cloned()
        .chain(additional)
        .flat_map(|pat| &pat.items)
        .map(|field| field.0.symbol.clone())
        .collect::<HashSet<_>>();
    let mut all_keys = all_keys.into_iter().collect::<Vec<_>>();
    all_keys.sort_by_key(|k| k.as_name());
    all_keys
}

fn is_wildcard(pat: &Pattern) -> bool {
    matches!(pat, Pattern::Identifier(_) | Pattern::Wildcard)
}
