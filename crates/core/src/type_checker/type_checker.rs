use std::collections::HashMap;

use crate::analyzer::session::Session;
use crate::analyzer::{ModuleId, ModulePath};
use crate::diagnostics::{Diagnostic, DiagnosticKind, DiagnosticLevel};
use crate::type_checker::analysis_context::{LocalContext, SymbolRef};
use crate::type_checker::SymbolHandle;
use crate::types::{self, Type, TypeId, TypeParam};
use crate::{ir, Location};

pub struct CheckResult {
    pub ir: ir::Program,
    pub symbols: Vec<SymbolHandle>,
    pub exports: Vec<SymbolRef>,
    pub diagnostics: Vec<Diagnostic>,
}
impl Default for CheckResult {
    fn default() -> Self {
        Self {
            ir: ir::Program { statements: vec![] },
            symbols: vec![],
            exports: vec![],
            diagnostics: vec![],
        }
    }
}

pub struct TypeChecker<'sess> {
    current_module: ModuleId,
    pub(crate) session: &'sess Session,
    pub diagnostics: Vec<Diagnostic>,
    pub ctx: LocalContext,
}

impl TypeChecker<'_> {
    pub fn new<'sess>(session: &'sess Session, id: ModuleId) -> TypeChecker<'sess> {
        TypeChecker {
            current_module: id,
            session,
            diagnostics: vec![],
            ctx: LocalContext::new(),
        }
    }

    pub fn get_file_name(&self) -> ModulePath {
        self.session.read_module(self.current_module).name.clone()
    }

    pub fn check(mut self) -> CheckResult {
        let ast = self.session.get_ast(self.current_module);
        let program = ir::Program {
            statements: ast
                .items
                .iter()
                .flat_map(|i| self.visit_item(i.clone()))
                .collect(),
        };

        CheckResult {
            ir: program,
            symbols: self.ctx.symbols,
            exports: self.ctx.scopes[0].bindings.clone(),
            diagnostics: self.diagnostics,
        }
    }

    pub fn intern(&self, ty: impl Into<Type>) -> TypeId {
        self.session.intern(ty.into())
    }
    pub fn intern_unique(&self, ty: Type) -> TypeId {
        self.session.intern_unique(ty)
    }

    pub fn resolve(&self, id: TypeId) -> Type {
        self.session.get_type(id)
    }

    pub fn can_be_assigned_to(&self, test_id: TypeId, against: TypeId) -> bool {
        let test = self.resolve(test_id);
        let against = self.resolve(against);
        match (&test, &against) {
            (Type::Unknown, _) => true,
            (_, Type::Unknown) => true,
            (_, Type::Duck(duck)) => self.session.types().implements(test_id, duck),
            (_, _) => test == against,
        }
    }

    pub fn with_scope<F, T>(&mut self, predicate: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        self.ctx.enter_scope();
        let res = predicate(self);
        let scope = self.ctx.pop_scope();
        self.ctx.add_dependencies(scope.captured());
        res
    }

    /// Execute the given predicate while registering outer dependencies (=enclosed variables)
    pub fn with_dependencies<F, T>(&mut self, predicate: F) -> (T, Vec<SymbolRef>)
    where
        F: FnOnce(&mut Self) -> T,
    {
        let memo = self.ctx.current_declaration_dependencies.clone();
        self.ctx.current_declaration_dependencies = Some(vec![]);
        let res = predicate(self);
        let dependencies = self.ctx.current_declaration_dependencies.clone().unwrap();
        self.ctx.current_declaration_dependencies = memo;
        (res, dependencies)
    }

    /// Returns how many dependencies were actually reactive
    pub fn save_reactive_dependencies(&mut self, deps: &Vec<SymbolRef>, at: Location) -> usize {
        let deps: Vec<SymbolRef> = deps
            .into_iter()
            .filter(|dep| self.resolve(dep.borrow().get_type()).is_reactive())
            .cloned()
            .collect();
        let len = deps.len();
        if len > 0 {
            self.ctx.other_dependencies.insert(at, deps);
        }
        len
    }

    pub fn error(&mut self, kind: DiagnosticKind, loc: Location) {
        self.diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Error,
            loc,
            kind,
        });
    }

    pub fn lookup(&self, name: &str) -> Option<SymbolRef> {
        self.ctx.lookup(name).or(self.session.find_builtin(name))
    }

    pub fn lookup_mut(&self, name: &str) -> Option<SymbolHandle> {
        let Some(symbol) = self.lookup(name) else {
            return None;
        };
        let local_symbol = self
            .ctx
            .symbols
            .iter()
            .find(|s| s.has_ref(&symbol))
            .cloned();
        if local_symbol.is_some() {
            local_symbol
        } else {
            self.session.get_handle(symbol)
        }
    }

    pub fn get_handle(&self, symbol: SymbolRef) -> Option<SymbolHandle> {
        self.ctx
            .symbols
            .iter()
            .find(|s| s.has_ref(&symbol))
            .cloned()
            .or_else(|| self.session.get_handle(symbol))
    }

    pub fn resolve_type_symbol(&self, ty: TypeId) -> Option<SymbolRef> {
        self.ctx
            .symbols
            .iter()
            .find(|s| self.equals_type(s.borrow().ty, ty) && s.borrow().is_type_symbol())
            .map(|s| s.readonly())
            .or_else(|| {
                self.session
                    .symbols()
                    .iter()
                    .find(|s| self.equals_type(s.borrow().ty, ty) && s.borrow().is_type_symbol())
                    .cloned()
            })
    }

    fn equals_type(&self, tested: TypeId, against: TypeId) -> bool {
        let tested = self.unwrap_generic(tested);
        let against = self.unwrap_generic(against);
        tested == against
    }
    /// If the given type id refers to a generic, unwrap the underlying type definition.
    /// Else, just return the original value.
    fn unwrap_generic(&self, ty: TypeId) -> TypeId {
        match self.resolve(ty) {
            types::Type::Generic(g) => g.definition,
            _ => ty,
        }
    }

    pub fn unify(
        &mut self,
        expected: TypeId,
        actual: TypeId,
        loc: Location,
        substitutions: &mut HashMap<TypeParam, TypeId>,
    ) {
        match (self.resolve(expected), self.resolve(actual)) {
            (Type::Param(p), a) => {
                match substitutions.get(&p) {
                    Some(p) => {
                        self.check_assigned_type(*p, actual, loc);
                    }
                    None => match &a {
                        Type::Param(_) => {}
                        _ => {
                            substitutions.insert(p, actual);
                        }
                    },
                };
            }
            (Type::Array(e), Type::Array(a)) => {
                self.unify(e.element, a.element, loc, substitutions);
            }
            (Type::Function(e), Type::Function(a)) => {
                if e.params.len() != a.params.len() {
                    let error = DiagnosticKind::MismatchedTypes {
                        left_name: self.session.display_type(expected),
                        right_name: self.session.display_type(actual),
                    };
                    self.error(error, loc);
                    return;
                }
                for (e, a) in e.params.iter().zip(a.params.iter()) {
                    self.unify(*e, *a, loc, substitutions);
                }
                self.unify(e.return_type, a.return_type, loc, substitutions);
            }
            (Type::Generic(e), Type::Generic(a)) => {
                for (e, a) in e.params.iter().zip(a.params.iter()) {
                    self.unify(*e, *a, loc, substitutions);
                }
            }
            (Type::Listener(e), Type::Listener(a)) => {
                self.unify(e.inner, a.inner, loc, substitutions);
            }
            (Type::Map(e), Type::Map(a)) => {
                self.unify(e.key, a.key, loc, substitutions);
                self.unify(e.value, a.value, loc, substitutions);
            }
            (Type::Option(e), Type::Option(a)) => {
                self.unify(e.some, a.some, loc, substitutions);
            }
            (Type::Reference(e), Type::Reference(a)) => {
                self.unify(e.target, a.target, loc, substitutions);
            }
            (Type::Result(e), Type::Result(a)) => {
                self.unify(e.ok, a.ok, loc, substitutions);
                match (&e.error, &a.error) {
                    (Some(e), Some(a)) => {
                        self.unify(*e, *a, loc, substitutions);
                    }
                    (None, None) => {}
                    _ => {
                        let error = DiagnosticKind::MismatchedTypes {
                            left_name: self.session.display_type(expected),
                            right_name: self.session.display_type(actual),
                        };
                        self.error(error, loc);
                    }
                }
            }
            (Type::Signal(e), Type::Signal(a)) => {
                self.unify(e.inner, a.inner, loc, substitutions);
            }
            (Type::Tuple(e), Type::Tuple(a)) => {
                if e.elements.len() != a.elements.len() {
                    let error = DiagnosticKind::MismatchedTypes {
                        left_name: self.session.display_type(expected),
                        right_name: self.session.display_type(actual),
                    };
                    self.error(error, loc);
                }
                for (e, a) in e.elements.iter().zip(a.elements.iter()) {
                    self.unify(*e, *a, loc, substitutions);
                }
            }
            (e, a) => {
                if e != a {
                    let error = DiagnosticKind::MismatchedTypes {
                        left_name: self.session.display_type(expected),
                        right_name: self.session.display_type(actual),
                    };
                    self.error(error, loc);
                }
            }
        }
    }
}
