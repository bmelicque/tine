use std::collections::{HashMap, HashSet};

use tine_ir as ir;
use tine_symbols::symbols::*;
use tine_types::store::TypeStore;

use crate::TypeChecker;

pub struct UsefulnessChecker<'tc> {
    tc: &'tc TypeChecker,
    ctor_arity: HashMap<Ctor, usize>,
    type_ctors: HashMap<TypeSymbolId, Option<Vec<Ctor>>>,
}
impl<'tc> UsefulnessChecker<'tc> {
    pub fn new(tc: &'tc TypeChecker) -> Self {
        Self {
            tc,
            ctor_arity: HashMap::new(),
            type_ctors: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct Ctor(TypeSymbolId, String);

#[derive(Debug, Clone)]
enum UsefulnessPattern {
    Wildcard,
    Ctor(Option<Ctor>, Vec<UsefulnessPattern>),
}
pub fn display_pattern(uc: &UsefulnessChecker, pattern: &UsefulnessPattern) -> String {
    match pattern {
        UsefulnessPattern::Wildcard => "_".to_string(),
        UsefulnessPattern::Ctor(ctor, args) => {
            let ctor = match ctor {
                Some(ctor) => ctor.1.clone(),
                None => "".to_string(),
            };
            let args = args
                .into_iter()
                .map(|a| display_pattern(uc, a))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({})", ctor, args)
        }
    }
}

fn to_useful(uc: &mut UsefulnessChecker, pattern: &ir::Pattern) -> UsefulnessPattern {
    use ir::Pattern::*;
    match pattern {
        Boolean(b) => bool_to_useful(uc, b),
        Float(f) => float_to_useful(uc, f),
        Integer(i) => int_to_useful(uc, i),
        String(s) => string_to_useful(uc, s),

        Call(c) => call_to_useful(uc, c),
        Identifier(_) => UsefulnessPattern::Wildcard,
        Struct(s) => struct_to_useful(uc, s),
        Tuple(t) => tuple_to_useful(uc, t),
    }
}
fn bool_to_useful(uc: &mut UsefulnessChecker, pattern: &ir::BooleanLiteral) -> UsefulnessPattern {
    let str = if pattern.value { "true" } else { "false" };
    let ty = uc.tc.get_type_symbol_id(TypeStore::BOOLEAN).unwrap();
    let ctor = Ctor(ty, str.into());
    if uc.ctor_arity.get(&ctor).is_none() {
        uc.ctor_arity.insert(ctor.clone(), 0);
    }
    if uc.type_ctors.get(&ty).is_none() {
        uc.type_ctors.insert(
            ty,
            Some(vec![Ctor(ty, "true".into()), Ctor(ty, "false".into())]),
        );
    }
    UsefulnessPattern::Ctor(Some(ctor), vec![])
}
fn float_to_useful(uc: &mut UsefulnessChecker, pattern: &ir::FloatLiteral) -> UsefulnessPattern {
    let str = pattern.value.to_string();
    let ty = uc.tc.get_type_symbol_id(TypeStore::FLOAT).unwrap();
    let ctor = Ctor(ty, str.into());
    if uc.ctor_arity.get(&ctor).is_none() {
        uc.ctor_arity.insert(ctor.clone(), 0);
    }
    uc.type_ctors.insert(ty, None);
    UsefulnessPattern::Ctor(Some(ctor), vec![])
}
fn int_to_useful(uc: &mut UsefulnessChecker, pattern: &ir::IntLiteral) -> UsefulnessPattern {
    let str = pattern.value.to_string();
    let ty = uc.tc.get_type_symbol_id(TypeStore::INTEGER).unwrap();
    let ctor = Ctor(ty, str.into());
    if uc.ctor_arity.get(&ctor).is_none() {
        uc.ctor_arity.insert(ctor.clone(), 0);
    }
    uc.type_ctors.insert(ty, None);
    UsefulnessPattern::Ctor(Some(ctor), vec![])
}
fn string_to_useful(uc: &mut UsefulnessChecker, pattern: &ir::StringLiteral) -> UsefulnessPattern {
    let str = pattern.value.clone();
    let ty = uc.tc.get_type_symbol_id(TypeStore::STRING).unwrap();
    let ctor = Ctor(ty, str.into());
    if uc.ctor_arity.get(&ctor).is_none() {
        uc.ctor_arity.insert(ctor.clone(), 0);
    }
    uc.type_ctors.insert(ty, None);
    UsefulnessPattern::Ctor(Some(ctor), vec![])
}
fn call_to_useful(uc: &mut UsefulnessChecker, pattern: &ir::CallPattern) -> UsefulnessPattern {
    let variant = pattern.callee.1;
    let name = uc.tc.symbol_name(variant).to_string();
    let e = uc.tc.symbols.get(variant).owner;
    let ctor = Ctor(e.into(), name);
    if uc.ctor_arity.get(&ctor).is_none() {
        let arity = uc.tc.symbols.get(variant).body.len();
        uc.ctor_arity.insert(ctor.clone(), arity);
    }
    if uc.type_ctors.get(&ctor.0).is_none() {
        let ctors = uc
            .tc
            .symbols
            .get(e)
            .variants
            .iter()
            .map(|v| Ctor(ctor.0, uc.tc.symbol_name(*v).to_string()))
            .collect();
        uc.type_ctors.insert(ctor.0, Some(ctors));
    }
    let arity = uc.ctor_arity[&ctor];
    let args = (0..arity)
        .into_iter()
        .map(|i| call_arg_to_useful(uc, &pattern.arguments, i))
        .collect();
    UsefulnessPattern::Ctor(Some(ctor), args)
}
fn call_arg_to_useful(
    uc: &mut UsefulnessChecker,
    args: &Vec<ir::Pattern>,
    i: usize,
) -> UsefulnessPattern {
    match args.get(i) {
        Some(a) => to_useful(uc, a),
        None => UsefulnessPattern::Wildcard,
    }
}

fn struct_to_useful(uc: &mut UsefulnessChecker, pattern: &ir::StructPattern) -> UsefulnessPattern {
    let members = &uc.tc.symbols.get(pattern.name.1).members;
    let patterns = members
        .iter()
        .map(|m| get_struct_field_pattern(uc, *m, &pattern.fields))
        .collect();
    UsefulnessPattern::Ctor(None, patterns)
}
fn get_struct_field_pattern(
    uc: &mut UsefulnessChecker,
    member: MemberSymbolId,
    fields: &[ir::StructPatternField],
) -> UsefulnessPattern {
    let name = uc.tc.symbol_name(member);
    let field = fields
        .into_iter()
        .find(|f| uc.tc.symbol_name(f.identifier.symbol) == name);
    let Some(field) = field else {
        return UsefulnessPattern::Wildcard;
    };
    match &field.pattern {
        Some(p) => to_useful(uc, p),
        None => UsefulnessPattern::Wildcard,
    }
}

fn tuple_to_useful(uc: &mut UsefulnessChecker, pattern: &ir::TuplePattern) -> UsefulnessPattern {
    let elements = pattern.elements.iter().map(|e| to_useful(uc, e)).collect();
    UsefulnessPattern::Ctor(None, elements)
}

type Row = Vec<UsefulnessPattern>;
type Matrix = Vec<Row>;

/// Specialize the matrix, ie keeps only rows compatible with constructor in first column.
fn specialize(ctor: &Option<Ctor>, arity: usize, matrix: &Matrix) -> Matrix {
    let mut out = Matrix::new();
    for row in matrix {
        match &row[0] {
            UsefulnessPattern::Wildcard => {
                let mut new_row = vec![UsefulnessPattern::Wildcard; arity];
                new_row.extend_from_slice(&row[1..]);
                out.push(new_row);
            }
            UsefulnessPattern::Ctor(c, args) if ctor == c => {
                let mut new_row = args.clone();
                new_row.extend_from_slice(&row[1..]);
                out.push(new_row);
            }
            _ => { /* different constructor: drop row */ }
        }
    }
    out
}

/// "default" matrix. Keeps only rows with a wildcard in column 1 and drops
/// that column. Used when column 1's constructors don't cover the
/// whole type, so a wildcard query might "fall through" via some other,
/// unlisted constructor.
fn default_matrix(matrix: &Matrix) -> Matrix {
    matrix
        .iter()
        .filter(|row| matches!(row[0], UsefulnessPattern::Wildcard))
        .map(|row| row[1..].to_vec())
        .collect()
}

/// The constructors actually appearing in column 1, plus (if any appear)
/// the full constructor set of their type, if it is a closed type.
fn column_signature(
    uc: &UsefulnessChecker,
    matrix: &Matrix,
) -> (HashSet<Option<Ctor>>, Option<Vec<Option<Ctor>>>) {
    let mut present = HashSet::new();
    for row in matrix {
        if let UsefulnessPattern::Ctor(ctor, _) = &row[0] {
            present.insert(ctor.clone());
        }
    }
    let full = present
        .iter()
        .next()
        .and_then(|c| constructor_signature(uc, c));
    (present, full)
}
fn constructor_signature(uc: &UsefulnessChecker, c: &Option<Ctor>) -> Option<Vec<Option<Ctor>>> {
    c.clone()
        .map_or(Some(vec![None]), |c| some_constructor_signature(uc, c))
}
fn some_constructor_signature(uc: &UsefulnessChecker, c: Ctor) -> Option<Vec<Option<Ctor>>> {
    uc.type_ctors[&c.0]
        .clone()
        .map(|ctors| ctors.into_iter().map(|c| Some(c)).collect())
}

/// The core algorithm. Returns every "witness" row demonstrating that
/// `query` is useful against `matrix` -- i.e. value-vectors matched by
/// `query` but by no row of `matrix`. An empty result means `query` is
/// *not* useful (everything it covers is already covered).
pub fn usefulness(
    uc: &mut UsefulnessChecker,
    matrix: &Matrix,
    query: &[UsefulnessPattern],
) -> Matrix {
    // Rule 1: zero columns left.
    if query.is_empty() {
        return if matrix.is_empty() {
            vec![vec![]]
        } else {
            vec![]
        };
    }

    match &query[0] {
        // Rule 2: query's first pattern is a concrete constructor.
        UsefulnessPattern::Ctor(ctor, args) => ctor_usefulness(uc, ctor, args, matrix, query),

        // Rule 3: query's first pattern is a wildcard.
        UsefulnessPattern::Wildcard => wildcard_usefulness(uc, matrix, query),
    }
}
fn ctor_usefulness(
    uc: &mut UsefulnessChecker,
    ctor: &Option<Ctor>,
    args: &Vec<UsefulnessPattern>,
    matrix: &Matrix,
    query: &[UsefulnessPattern],
) -> Matrix {
    let arity = args.len();
    let spec = specialize(ctor, arity, matrix);
    let mut sub_query = args.clone();
    sub_query.extend_from_slice(&query[1..]);
    usefulness(uc, &spec, &sub_query)
        .into_iter()
        .map(|w| ctor_witness(ctor.clone(), arity, w))
        .collect()
}
fn ctor_witness(ctor: Option<Ctor>, arity: usize, w: Vec<UsefulnessPattern>) -> Row {
    let (head, tail) = w.split_at(arity);
    let mut row = vec![UsefulnessPattern::Ctor(ctor, head.to_vec())];
    row.extend_from_slice(tail);
    row
}

fn wildcard_usefulness(
    uc: &mut UsefulnessChecker,
    matrix: &Matrix,
    query: &[UsefulnessPattern],
) -> Matrix {
    let (present, full) = column_signature(uc, matrix);
    let is_complete = match &full {
        Some(all) => all.iter().all(|c| present.contains(c)),
        None => false,
    };

    if is_complete {
        wildcard_usefulness_on_complete_signature(uc, matrix, query, full.unwrap())
    } else {
        // Some constructor is missing from the matrix (or the type
        // is open/infinite) -- the wildcard "escapes" through it, so
        // only the already-wildcard rows are relevant.
        let def = default_matrix(matrix);
        usefulness(uc, &def, &query[1..])
            .into_iter()
            .map(|w| {
                let mut row = vec![UsefulnessPattern::Wildcard];
                row.extend_from_slice(&w);
                row
            })
            .collect()
    }
}
fn wildcard_usefulness_on_complete_signature(
    uc: &mut UsefulnessChecker,
    matrix: &Matrix,
    query: &[UsefulnessPattern],
    all: Vec<Option<Ctor>>,
) -> Matrix {
    // Every constructor of the type must be checked individually.
    let mut results = Vec::new();
    for c in &all {
        let arity = match c {
            Some(c) => uc.ctor_arity[c],
            None => matrix
                .iter()
                .find_map(|row| match &row[0] {
                    UsefulnessPattern::Ctor(head, args) if head == c => Some(args.len()),
                    _ => None,
                })
                .expect("constructor is in signature so a representative row must exist"),
        };
        let spec = specialize(c, arity, matrix);
        let mut sub_query = vec![UsefulnessPattern::Wildcard; arity];
        sub_query.extend_from_slice(&query[1..]);
        for w in usefulness(uc, &spec, &sub_query) {
            let (head, tail) = w.split_at(arity);
            let mut row = vec![UsefulnessPattern::Ctor(c.clone(), head.to_vec())];
            row.extend_from_slice(tail);
            results.push(row);
        }
    }
    results
}

impl TypeChecker {
    pub fn usefulness<'ir>(
        &'ir self,
        uc: &mut UsefulnessChecker,
        matrix: &Vec<Vec<&'ir ir::Pattern>>,
        query: &[ir::Pattern],
    ) -> Matrix {
        let matrix = matrix
            .into_iter()
            .map(|row| row.iter().map(|p| to_useful(uc, p)).collect::<Vec<_>>())
            .collect();
        let query = query
            .into_iter()
            .map(|q| to_useful(uc, q))
            .collect::<Vec<_>>();
        usefulness(uc, &matrix, &query)
    }
}
