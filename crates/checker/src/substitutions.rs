use std::collections::HashMap;

use tine_common::{diagnostics::DiagnosticKind, locations::Location};
use tine_types::{
    store::{display_type, TypeStore},
    types,
};

use crate::TypeChecker;

pub type SubstitutionTable = HashMap<types::TypeParam, types::TypeId>;
impl From<Substitutions> for SubstitutionTable {
    fn from(value: Substitutions) -> Self {
        value.table
    }
}
impl From<&Substitutions> for SubstitutionTable {
    fn from(value: &Substitutions) -> Self {
        value.table.clone()
    }
}

#[derive(Debug, Default, Clone)]
pub struct Substitutions {
    table: SubstitutionTable,
    mismatched_pairs: Vec<(types::TypeId, types::TypeId)>,
    diagnostics: Vec<DiagnosticKind>,
}
impl From<SubstitutionTable> for Substitutions {
    fn from(table: SubstitutionTable) -> Self {
        Self {
            table,
            ..Default::default()
        }
    }
}

impl Substitutions {
    pub fn new() -> Self {
        Self {
            table: HashMap::new(),
            mismatched_pairs: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn with_initial(params: &[types::TypeParam], args: &[types::TypeId]) -> Self {
        let mut sub = Self::new();
        sub.table = params
            .iter()
            .zip(args)
            .map(|(p, a)| (p.clone(), *a))
            .collect();
        sub
    }

    /// Find all the substitutions that would make the given generic equal to
    /// the given concrete type.
    pub fn unify(
        &mut self,
        tc: &mut TypeChecker,
        generic: types::TypeId,
        concrete: types::TypeId,
        loc: Location,
    ) {
        let g = tc.types.get(generic).clone();
        let c = tc.types.get(concrete).clone();
        use types::Type::*;
        match (g, c) {
            (Param(p), a) => {
                match self.table.get(&p) {
                    Some(&p) => {
                        if !tc.can_be_assigned_to(concrete, p, true) {
                            self.push_mismatched(&tc.types, p, concrete);
                        }
                    }
                    None => match &a {
                        Param(_) => {}
                        _ => {
                            self.table.insert(p.clone(), concrete);
                        }
                    },
                };
            }
            (Function(e), Function(a)) => {
                if e.params.len() != a.params.len() {
                    self.mismatched_pairs.push((generic, concrete));
                    return;
                }
                for (e, a) in e.params.iter().zip(a.params.iter()) {
                    self.unify(tc, *e, *a, loc);
                }
                self.unify(tc, e.return_type, a.return_type, loc);
            }
            (Listener(e), Listener(a)) => {
                self.unify(tc, e.inner, a.inner, loc);
            }
            (Option(e), Option(a)) => {
                self.unify(tc, e.some, a.some, loc);
            }
            (Result(e), Result(a)) => {
                self.unify(tc, e.ok, a.ok, loc);
                match (&e.error, &a.error) {
                    (Some(e), Some(a)) => {
                        self.unify(tc, *e, *a, loc);
                    }
                    (None, None) => {}
                    _ => {
                        self.mismatched_pairs.push((generic, concrete));
                    }
                }
            }
            (Signal(e), Signal(a)) => {
                self.unify(tc, e.inner, a.inner, loc);
            }
            (Tuple(e), Tuple(a)) => {
                if e.elements.len() != a.elements.len() {
                    self.mismatched_pairs.push((generic, concrete));
                }
                for (e, a) in e.elements.iter().zip(a.elements.iter()) {
                    self.unify(tc, *e, *a, loc);
                }
            }
            (e, a) => {
                if e != a {
                    self.mismatched_pairs.push((generic, concrete));
                }
            }
        }
    }

    /// Apply current substitutions to a type (if generic)
    pub fn apply(&self, store: &mut TypeStore, to: types::TypeId) -> types::TypeId {
        use types::Type::*;
        match store.get(to).clone() {
            Boolean => to,
            Dynamic => todo!(),
            Enum(e) => {
                let args = self.resolve_params(&e.params);
                store.add(types::TypeRef { inner: e.id, args })
            }
            Float => to,
            Function(mut f) => {
                f.params = f.params.into_iter().map(|p| self.apply(store, p)).collect();
                f.return_type = self.apply(store, f.return_type);
                store.add(f)
            }
            Generic(g) => {
                let args = self.resolve_params(&g.params);
                store.add(types::TypeRef { inner: to, args })
            }
            Integer => to,
            Listener(mut l) => {
                l.inner = self.apply(store, l.inner);
                store.add(l)
            }
            Option(mut o) => {
                o.some = self.apply(store, o.some);
                store.add(o)
            }
            Param(p) => *self.table.get(&p).unwrap_or(&to),
            Ref(_) => panic!("already resolved"),
            Result(mut r) => {
                r.ok = self.apply(store, r.ok);
                match &r.error {
                    Some(e) => r.error = Some(self.apply(store, *e)),
                    None => {}
                }
                store.add(r)
            }
            SelfType => to,
            Signal(mut s) => {
                s.inner = self.apply(store, s.inner);
                store.add(s)
            }
            String => to,
            Struct(s) => {
                let args = self.resolve_params(&s.params);
                store.add(types::TypeRef { inner: s.id, args })
            }
            Trait(t) => {
                let args = self.resolve_params(&t.params);
                store.add(types::TypeRef { inner: to, args })
            }
            Tuple(mut t) => {
                t.elements = t
                    .elements
                    .into_iter()
                    .map(|e| self.apply(store, e))
                    .collect();
                store.add(t)
            }
            Unit => to,
            Unknown => to,
        }
    }
    fn resolve_params(&self, params: &[types::TypeParam]) -> Vec<types::TypeId> {
        params
            .iter()
            .map_while(|p| self.table.get(p))
            .cloned()
            .collect()
    }

    fn push_mismatched(&mut self, store: &TypeStore, left: types::TypeId, right: types::TypeId) {
        let diag = DiagnosticKind::MismatchedTypes {
            left_name: display_type(store, left),
            right_name: display_type(store, right),
        };
        self.diagnostics.push(diag);
    }

    pub fn produce_diagnostics(&self, store: &TypeStore) -> Vec<DiagnosticKind> {
        self.mismatched_pairs
            .clone()
            .into_iter()
            .map(|(l, r)| DiagnosticKind::MismatchedTypes {
                left_name: display_type(store, l),
                right_name: display_type(store, r),
            })
            .collect()
    }
}
