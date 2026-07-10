use std::{collections::HashSet, sync::LazyLock};

use tine_common::locations::Location;
use tine_ir as ir;
use tine_symbols::symbols::*;
use tine_types::store::TypeStore;

use crate::{
    patterns::{
        ConstructorPattern, LiteralPattern, Pattern, PatternField, StructPattern, TuplePattern,
    },
    TypeChecker,
};

static DEFAULT_STRUCT: LazyLock<StructPattern> = LazyLock::new(StructPattern::default);
static DEFAULT_TUPLE: LazyLock<TuplePattern> = LazyLock::new(TuplePattern::default);

type Row<'p> = Vec<&'p Pattern>;
type Matrix<'p> = Vec<Row<'p>>;

struct Specialization<'p> {
    new_arms: Matrix<'p>,
    new_query: Row<'p>,
}

impl TypeChecker {
    fn specialize_constructor<'p>(
        &self,
        arms: &Matrix<'p>,
        query_head: &'p ConstructorPattern,
        query_tail: &[&'p Pattern],
    ) -> Specialization<'p> {
        let expects_arg = match query_head.identifier.symbol {
            SymbolId::Variant(s) => self.symbols.get(s).body.is_some(),
            _ => panic!(),
        };
        let new_arms = arms
            .into_iter()
            .filter_map(|arm| {
                let (head, tail) = arm.split_first()?;
                head.as_constructor().map(|lit| (lit, tail))
            })
            .filter(|(head, _)| head.identifier.symbol == query_head.identifier.symbol)
            .map(|(head, tail)| {
                if expects_arg {
                    let mut tail = tail.to_vec();
                    tail.insert(0, head.arg.as_deref().unwrap_or(&Pattern::Wildcard));
                    tail
                } else {
                    tail.to_vec()
                }
            })
            .collect();

        let new_query = if expects_arg {
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
        &self,
        arms: &'p Matrix<'p>,
        query_head: &'p StructPattern,
        query_tail: &'p [&'p Pattern],
    ) -> (Specialization<'p>, Vec<SymbolId>) {
        let structs = get_structs(arms);
        let all_keys = self.all_keys_from_structs(&structs, Some(query_head));

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
        &self,
        arms: &'p Matrix<'p>,
        query_head: &'p TuplePattern,
        query_tail: &'p [&'p Pattern],
    ) -> (Specialization<'p>, Vec<SymbolId>) {
        let tuples = get_tuples(arms);
        let all_keys = self.all_keys_from_tuples(&tuples, Some(query_head));

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

    /// Returns a list of patterns covered by the query that were not covered by
    /// any arm.
    pub fn usefulness(&self, arms: &Matrix, query: &[&Pattern]) -> Vec<Vec<Pattern>> {
        let (head, tail) = query.split_first().unwrap();
        match head {
            Pattern::Literal(p) => self.literal_usefulness(arms, p, tail),
            Pattern::Constructor(p) => self.constructor_usefulness(arms, p, tail),
            Pattern::Struct(p) => self.struct_usefulness(arms, p, tail),
            Pattern::Tuple(p) => self.tuple_usefulness(arms, p, tail),
            Pattern::Identifier(_) | Pattern::Wildcard => self.wildcard_usefulness(arms, tail),
        }
    }

    fn literal_usefulness(
        &self,
        arms: &Matrix,
        query_head: &LiteralPattern,
        query_tail: &[&Pattern],
    ) -> Vec<Vec<Pattern>> {
        let got = specialize_literal(arms, query_head, query_tail);
        if self.usefulness(&got.new_arms, &got.new_query).is_empty() {
            vec![vec![Pattern::Wildcard]]
        } else {
            vec![]
        }
    }

    fn constructor_usefulness(
        &self,
        arms: &Matrix,
        query_head: &ConstructorPattern,
        query_tail: &[&Pattern],
    ) -> Vec<Vec<Pattern>> {
        let expects_arg = match query_head.identifier.symbol {
            SymbolId::Variant(s) => self.symbols.get(s).body.is_some(),
            _ => panic!(),
        };
        let s = self.specialize_constructor(arms, query_head, query_tail);
        self.usefulness(&s.new_arms, &s.new_query)
            .into_iter()
            .map(|mut witness| {
                let (arg, mut tail) = if expects_arg {
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
        &self,
        arms: &Matrix,
        query_head: &StructPattern,
        query_tail: &[&Pattern],
    ) -> Vec<Vec<Pattern>> {
        let (s, keys) = self.specialize_struct(arms, query_head, query_tail);
        self.usefulness(&s.new_arms, &s.new_query)
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
        &self,
        arms: &Matrix,
        query_head: &TuplePattern,
        query_tail: &[&Pattern],
    ) -> Vec<Vec<Pattern>> {
        let (s, keys) = self.specialize_tuple(arms, query_head, query_tail);
        self.usefulness(&s.new_arms, &s.new_query)
            .into_iter()
            .map(|witness| {
                let (items, mut tail) = reconstruct_fields(witness, &keys);
                let pat = TuplePattern { items };
                tail.insert(0, pat.into());
                tail
            })
            .collect()
    }

    fn wildcard_usefulness(&self, arms: &Matrix, query_tail: &[&Pattern]) -> Vec<Vec<Pattern>> {
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

            return self.usefulness(&arms, query_tail);
        };

        match first {
            Pattern::Identifier(_) | Pattern::Wildcard => unreachable!(),

            Pattern::Literal(_) => self.wildcard_usefulness_literal(arms, query_tail),
            Pattern::Constructor(c) => self.wildcard_usefulness_constructor(arms, c, query_tail),
            Pattern::Struct(_) => self.wildcard_usefulness_struct(arms, query_tail),
            Pattern::Tuple(_) => self.wildcard_usefulness_tuple(arms, query_tail),
        }
    }
    fn wildcard_usefulness_literal(
        &self,
        arms: &Matrix,
        query_tail: &[&Pattern],
    ) -> Vec<Vec<Pattern>> {
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
            return self.usefulness(&arms, query_tail);
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
            .flat_map(|pat| self.literal_usefulness(arms, &pat.into(), query_tail))
            .collect()
    }
    fn wildcard_usefulness_constructor(
        &self,
        arms: &Matrix,
        example: &ConstructorPattern,
        query_tail: &[&Pattern],
    ) -> Vec<Vec<Pattern>> {
        let variant = example.identifier.symbol.as_variant().unwrap();
        let owner = self.symbols.get(variant).owner;
        let variants = &self.symbols.get(owner).variants;

        variants
            .into_iter()
            .flat_map(|&symbol| {
                let variant = self.symbols.get::<VariantSymbolId>(symbol);
                let arg = if variant.body.is_some() {
                    Some(Box::new(Pattern::Wildcard))
                } else {
                    None
                };
                let identifier = ir::Identifier {
                    loc: Location::dummy(),
                    symbol: symbol.into(),
                    ty: self.symbol_type_id(owner),
                };
                let pattern = ConstructorPattern { identifier, arg };
                self.constructor_usefulness(arms, &pattern, query_tail)
            })
            .collect()
    }
    fn wildcard_usefulness_struct(
        &self,
        arms: &Matrix,
        query_tail: &[&Pattern],
    ) -> Vec<Vec<Pattern>> {
        let query = self.get_struct_signature(arms);
        self.struct_usefulness(arms, &query, query_tail)
    }

    fn get_struct_signature(&self, arms: &Matrix) -> StructPattern {
        let structs = get_structs(arms);
        let all_keys = self.all_keys_from_structs(&structs, None);
        let fields = all_keys
            .into_iter()
            .map(|symbol| {
                PatternField(
                    ir::Identifier {
                        loc: Location::dummy(),
                        symbol,
                        ty: self.symbol_type_id(symbol),
                    },
                    Pattern::Wildcard,
                )
            })
            .collect();
        StructPattern { fields }
    }

    fn all_keys_from_structs(
        &self,
        structs: &[(&StructPattern, &[&Pattern])],
        additional: Option<&StructPattern>,
    ) -> Vec<SymbolId> {
        let all_keys = structs
            .iter()
            .map(|(head, _)| head)
            .cloned()
            .chain(additional)
            .flat_map(|pat| &pat.fields)
            .map(|field| field.0.symbol.clone())
            .collect::<HashSet<_>>();
        let mut all_keys = all_keys.into_iter().collect::<Vec<_>>();
        all_keys.sort_by_key(|k| self.symbol_name(*k));
        all_keys
    }

    fn wildcard_usefulness_tuple(
        &self,
        arms: &Matrix,
        query_tail: &[&Pattern],
    ) -> Vec<Vec<Pattern>> {
        let query = self.get_tuple_signature(arms);
        self.tuple_usefulness(arms, &query, query_tail)
    }

    fn get_tuple_signature(&self, arms: &Matrix) -> TuplePattern {
        let tuples = get_tuples(arms);
        let all_keys = self.all_keys_from_tuples(&tuples, None);
        let items = all_keys
            .into_iter()
            .map(|symbol| {
                PatternField(
                    ir::Identifier {
                        loc: Location::dummy(),
                        symbol,
                        ty: self.symbol_type_id(symbol),
                    },
                    Pattern::Wildcard,
                )
            })
            .collect();
        TuplePattern { items }
    }

    fn all_keys_from_tuples(
        &self,
        tuples: &[(&TuplePattern, &[&Pattern])],
        additional: Option<&TuplePattern>,
    ) -> Vec<SymbolId> {
        let all_keys = tuples
            .iter()
            .map(|(head, _)| head)
            .cloned()
            .chain(additional)
            .flat_map(|pat| &pat.items)
            .map(|field| field.0.symbol.clone())
            .collect::<HashSet<_>>();
        let mut all_keys = all_keys.into_iter().collect::<Vec<_>>();
        all_keys.sort_by_key(|k| self.symbol_name(*k));
        all_keys
    }
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

fn unwrap_fields<'p>(
    fields: &'p [PatternField],
    tail: &'p [&Pattern],
    sorted_keys: &[SymbolId],
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

/// Returns `(fields, witness_tail)`
fn reconstruct_fields(
    mut witness: Vec<Pattern>,
    keys: &[SymbolId],
) -> (Vec<PatternField>, Vec<Pattern>) {
    let tail = witness.split_off(keys.len());
    let fields = keys
        .iter()
        .zip(witness)
        .map(|(key, pattern)| {
            PatternField(
                ir::Identifier {
                    loc: Location::dummy(),
                    symbol: *key,
                    ty: TypeStore::UNKNOWN,
                },
                pattern,
            )
        })
        .collect();
    (fields, tail)
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

fn is_wildcard(pat: &Pattern) -> bool {
    matches!(pat, Pattern::Identifier(_) | Pattern::Wildcard)
}
