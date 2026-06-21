use crate::analyzer::session::Session;
use crate::analyzer::{ModuleId, ModulePath};
use crate::diagnostics::{Diagnostic, DiagnosticKind, DiagnosticLevel};
use crate::type_checker::analysis_context::{LocalContext, SymbolRef};
use crate::type_checker::SymbolHandle;
use crate::types::{self, Type, TypeId};
use crate::{ir, Location, SymbolKind};

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
    pub fn intern_unique(&self, ty: impl Into<Type>) -> TypeId {
        self.session.intern_unique(ty.into())
    }
    pub fn add_type_param(&self, name: String) -> types::TypeParam {
        let param = types::TypeParam {
            name: name.clone(),
            id: 0,
        };
        let id = self.intern_unique(param);
        types::TypeParam { name, id }
    }

    pub fn resolve(&self, id: TypeId) -> Type {
        self.session.get_type(id)
    }

    pub fn can_be_assigned_to(&self, got: TypeId, expected: TypeId) -> bool {
        self.session.types().can_assign_to(got, expected)
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

    /// Resolve the original symbol behind a type.
    ///
    /// If the given type is a type ref with type arguments, the function
    /// returns the generic definition of the type (ir either a struct, enum or
    /// generic alias).
    pub fn resolve_type_symbol(&self, mut ty: TypeId) -> Option<SymbolRef> {
        while let types::Type::Ref(r) = self.resolve(ty) {
            ty = r.inner
        }
        let r = self
            .ctx
            .symbols
            .iter()
            .filter(|s| s.borrow().is_type_symbol())
            .find(|s| s.borrow().ty == ty);
        if let Some(r) = r {
            return Some(r.readonly());
        }

        self.session
            .symbols()
            .into_iter()
            .filter(|s| s.borrow().is_type_symbol())
            .find(|s| s.borrow().ty == ty)
    }

    /// Given a value whose type implements the given trait, check if an
    /// immutable version of the type also implements the trait (since some
    /// methods might be defined with a mutable receiver).
    pub(super) fn immutable_implements_trait(
        &self,
        ty: types::TypeId,
        trait_: &types::TraitType,
    ) -> bool {
        // Node: `value.ty()` should implement `trait_`!
        if let Type::Trait(test) = &self.resolve(ty) {
            if *test == *trait_ {
                return true;
            }
        }

        let Some(type_symbol) = self.resolve_type_symbol(ty) else {
            panic!()
        };

        let value_methods = type_symbol
            .as_methods()
            .unwrap_or(vec![])
            .into_iter()
            .map(|m| {
                (
                    types::TraitMethod {
                        name: m.as_name(),
                        def: m.as_type(),
                    },
                    m,
                )
            })
            .collect::<Vec<_>>();
        for trait_method in &trait_.methods {
            let Some((_, value_method)) = value_methods.iter().find(|m| m.0 == *trait_method)
            else {
                panic!()
            };
            let SymbolKind::Method { receiver, .. } = &value_method.borrow().kind else {
                panic!()
            };
            if receiver.is_mutable() {
                return false;
            }
        }
        true
    }
}
