use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use tine_common::{
    diagnostics::{Diagnostic, DiagnosticKind, DiagnosticLevel},
    locations::{Locatable, Location},
    module_path::{ModuleId, ModulePath},
    sources::Source,
};
use tine_ir::{self as ir, Typed};
use tine_parser::ProjectParser;
use tine_symbols::{symbols::*, table::*};
use tine_types::{store::TypeStore, types};

use crate::{
    loader::{CheckerLoader, LoadedModule, MockLoader, ModuleLoader},
    substitutions::Substitutions,
};

#[derive(Debug, Default)]
pub struct CheckResult {
    pub ir: HashMap<ModuleId, ir::Program>,
    pub types: TypeStore,
    pub symbols: SymbolTable,
    pub diagnostics: HashMap<ModuleId, Vec<Diagnostic>>,
}

#[derive(Debug, Default)]
pub struct CheckProjectResult {
    pub names: Vec<ModulePath>,
    pub ids: HashMap<ModulePath, ModuleId>,

    pub sources: HashMap<ModuleId, Source>,
    pub ir: HashMap<ModuleId, ir::Program>,
    pub types: TypeStore,
    pub symbols: SymbolTable,
    pub diagnostics: HashMap<ModuleId, Vec<Diagnostic>>,
}

pub fn check_project(project: ProjectParser) -> CheckProjectResult {
    let sorted_modules = project.try_sorted_vec().unwrap();
    let names = project.names.clone();
    let ids = project.ids.clone();
    let loader = CheckerLoader {
        names: project.names,
        ids: project.ids,
        ast: project.ast,
    };
    let mut diagnostics = project.diagnostics;
    let mut tc = TypeChecker::with_loader(Box::new(loader));
    for module in sorted_modules {
        tc.check_module(module);
    }
    let r = tc.results();
    r.diagnostics.into_iter().for_each(|(id, d)| {
        diagnostics.entry(id).or_default().extend(d);
    });
    CheckProjectResult {
        names,
        ids,
        sources: project.sources,
        ir: r.ir,
        types: r.types,
        symbols: r.symbols,
        diagnostics,
    }
}

pub struct ScopeGuard<'tc> {
    tc: &'tc mut TypeChecker,
}
impl<'tc> ScopeGuard<'tc> {
    pub fn new(tc: &'tc mut TypeChecker) -> Self {
        Self { tc }
    }
}
impl Drop for ScopeGuard<'_> {
    fn drop(&mut self) {
        self.tc.drop_scope();
    }
}
impl Deref for ScopeGuard<'_> {
    type Target = TypeChecker;
    fn deref(&self) -> &Self::Target {
        self.tc
    }
}
impl DerefMut for ScopeGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.tc
    }
}

pub struct ThisGuard<'a> {
    pub tc: &'a mut TypeChecker,
}
impl Drop for ThisGuard<'_> {
    fn drop(&mut self) {
        self.tc.this.pop();
    }
}
impl Deref for ThisGuard<'_> {
    type Target = TypeChecker;
    fn deref(&self) -> &Self::Target {
        &self.tc
    }
}
impl DerefMut for ThisGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tc
    }
}

pub struct BindingGuard<'a> {
    previous: Option<types::TypeId>,
    pub tc: &'a mut TypeChecker,
}
impl Drop for BindingGuard<'_> {
    fn drop(&mut self) {
        self.tc.binding_expectation = self.previous;
    }
}
impl Deref for BindingGuard<'_> {
    type Target = TypeChecker;
    fn deref(&self) -> &Self::Target {
        &self.tc
    }
}
impl DerefMut for BindingGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.tc
    }
}

pub struct MutableThisGuard<'a> {
    previous: Option<bool>,
    pub tc: &'a mut TypeChecker,
}
impl Drop for MutableThisGuard<'_> {
    fn drop(&mut self) {
        self.tc.mutable_this = self.previous;
    }
}
impl Deref for MutableThisGuard<'_> {
    type Target = TypeChecker;
    fn deref(&self) -> &Self::Target {
        &self.tc
    }
}
impl DerefMut for MutableThisGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.tc
    }
}

pub struct TypeChecker {
    current_module: ModuleId,
    /// The global type store, that can be read and written through each
    /// module's type checker.
    pub(super) types: TypeStore,
    /// An arena for all the symbols (i.e. names) declared and defined accross
    /// the project.
    pub symbols: SymbolTable,
    pub(crate) scopes: Vec<Scope>,
    pub(crate) this: Vec<types::TypeId>,
    pub(crate) mutable_this: Option<bool>,
    pub(crate) placeholders: HashMap<types::Placeholder, types::TypeId>,
    pub(crate) binding_expectation: Option<types::TypeId>,

    pub(super) loader: Box<dyn ModuleLoader>,

    ir: HashMap<ModuleId, ir::Program>,
    exports: HashMap<ModuleId, HashMap<String, SymbolId>>,
    pub(super) diagnostics: HashMap<ModuleId, Vec<Diagnostic>>,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut tc = Self {
            current_module: 0,
            types: TypeStore::new(),
            symbols: SymbolTable::default(),
            scopes: vec![Scope::new()],
            this: vec![],
            mutable_this: None,
            placeholders: HashMap::new(),
            binding_expectation: None,

            loader: Box::new(MockLoader),

            ir: HashMap::new(),
            exports: HashMap::new(),
            diagnostics: HashMap::new(),
        };
        tc.init_internals();
        tc.init_builtins();
        tc
    }

    pub fn current_module(&self) -> ModuleId {
        self.current_module
    }

    pub fn with_loader(loader: Box<dyn ModuleLoader>) -> Self {
        let mut tc = Self::new();
        tc.loader = loader;
        tc
    }

    pub fn check_module(&mut self, module_id: ModuleId) {
        self.current_module = module_id;
        let module = match self.loader.module(module_id) {
            LoadedModule::Real(m) => m,
            LoadedModule::Virtual(m) => {
                self.delegated_check(m);
                return;
            }
        };
        self.scopes.push(Scope::new());
        let program = ir::Program {
            statements: module
                .items
                .iter()
                .flat_map(|i| self.visit_item(i.clone()))
                .collect(),
        };
        let scope = self.drop_scope();

        self.ir.insert(module_id, program);
        let exports = scope.as_bindings();
        self.exports.insert(module_id, exports);
    }

    pub fn delegated_check<F>(&mut self, cb: F)
    where
        F: Fn(&mut Self),
    {
        cb(self)
    }

    pub(super) fn add_exports(&mut self, id: ModuleId, exports: HashMap<String, SymbolId>) {
        self.exports.insert(id, exports);
    }

    pub fn results(self) -> CheckResult {
        CheckResult {
            ir: self.ir,
            types: self.types,
            symbols: self.symbols,
            diagnostics: self.diagnostics,
        }
    }

    pub(crate) fn insert<I>(&mut self, s: I::SymbolKind) -> I
    where
        I: SymbolIndex + Clone + Into<SymbolId>,
    {
        let name = s.name().to_string();
        let id: I = self.symbols.insert(s);
        self.current_scope().bind(name, id.clone().into());
        id
    }

    pub fn intern(&mut self, ty: impl Into<types::Type>) -> types::TypeId {
        self.types.add(ty.into())
    }
    pub fn intern_unique(&mut self, ty: impl Into<types::Type>) -> types::TypeId {
        self.types.add_unique(ty.into())
    }
    pub fn add_type_param(&mut self, name: String) -> types::TypeParam {
        let param = types::TypeParam {
            name: name.clone(),
            id: 0,
        };
        let id = self.intern_unique(param);
        types::TypeParam { name, id }
    }

    pub fn resolve(&self, id: types::TypeId) -> types::Type {
        self.types.get(id).clone()
    }

    pub fn symbol_type_id<S>(&self, symbol: S) -> types::TypeId
    where
        S: Into<SymbolId>,
    {
        let symbol = symbol.into();
        self.symbols.get_symbol(symbol).ty()
    }

    pub fn symbol_type<S>(&self, symbol: S) -> types::Type
    where
        S: Into<SymbolId>,
    {
        let symbol = symbol.into();
        let ty = self.symbols.get_symbol(symbol).ty();
        self.types.get(ty).clone()
    }

    pub fn symbol_name<'s, S>(&'s self, symbol: S) -> &'s str
    where
        S: Into<SymbolId>,
    {
        let symbol = symbol.into();
        self.symbols.get_symbol(symbol).name()
    }

    pub fn read_symbol<'s, S>(&'s mut self, symbol: S, at: Location)
    where
        S: Into<SymbolId>,
    {
        let symbol = symbol.into();
        let access = self.symbols.get_symbol_mut(symbol).access();
        access.read(at);
    }

    pub fn symbol_methods<'s, S>(&'s self, symbol: S) -> &'s [MethodSymbolId]
    where
        S: Into<TypeSymbolId>,
    {
        let symbol = symbol.into();
        match symbol {
            TypeSymbolId::Enum(s) => &self.symbols.get(s).methods,
            TypeSymbolId::Primitive(s) => &self.symbols.get(s).methods,
            TypeSymbolId::Struct(s) => &self.symbols.get(s).methods,
        }
    }
    pub fn symbol_methods_mut<'s, S>(&'s mut self, symbol: S) -> &'s mut Vec<MethodSymbolId>
    where
        S: Into<TypeSymbolId>,
    {
        let symbol = symbol.into();
        match symbol {
            TypeSymbolId::Enum(s) => &mut self.symbols.get_mut(s).methods,
            TypeSymbolId::Primitive(s) => &mut self.symbols.get_mut(s).methods,
            TypeSymbolId::Struct(s) => &mut self.symbols.get_mut(s).methods,
        }
    }

    pub fn builtin_id<I>(&self, name: &str) -> Option<I>
    where
        I: SymbolIndex,
    {
        self.symbols
            .find_id::<I, _>(|s| s.name() == name && s.defined_at().module() == 0)
    }
    pub fn builtin_symbol(&self, name: &str) -> Option<&dyn Symbol> {
        self.symbols
            .all()
            .map(|s| s.1)
            .filter(|s| s.defined_at().module() == 0)
            .find(|s| s.name() == name)
    }

    pub fn can_be_assigned_to(
        &mut self,
        got: types::TypeId,
        expected_id: types::TypeId,
        got_immutable: bool,
    ) -> bool {
        let actual = self.types.get(got).clone();
        let expected = self.types.get(expected_id).clone();
        use types::Type::*;
        match (expected, actual) {
            (Unknown, _) | (_, Unknown) => true,

            (Placeholder(p), _) => match self.placeholders.get(&p).cloned() {
                Some(t) => self.can_be_assigned_to(got, t, got_immutable),
                None => {
                    self.placeholders.insert(p, got);
                    true
                }
            },

            (_, Placeholder(p)) => match self.placeholders.get(&p).cloned() {
                Some(t) => self.can_be_assigned_to(t, expected_id, got_immutable),
                None => {
                    self.placeholders.insert(p, expected_id);
                    true
                }
            },

            (Trait(t), _) => self.implements_trait(got, &t.clone(), got_immutable),
            (Ref(e), Ref(a)) => e
                .args
                .iter()
                .zip(&a.args)
                .all(|(e, a)| self.can_be_assigned_to(*a, *e, got_immutable)),
            (e, Ref(a)) if e.is_generic() => a.inner == expected_id,
            (Float, Integer) => true,

            (Function(expected), Function(actual)) => {
                let params_ok = expected
                    .params
                    .into_iter()
                    .zip(actual.params)
                    .all(|(e, a)| self.can_be_assigned_to(a, e, got_immutable));
                params_ok
                    && self.can_be_assigned_to(
                        actual.return_type,
                        expected.return_type,
                        got_immutable,
                    )
            }

            (Tuple(e), Tuple(a)) => {
                if e.elements.len() != a.elements.len() {
                    return false;
                }
                e.elements
                    .into_iter()
                    .zip(a.elements)
                    .all(|(e, a)| self.can_be_assigned_to(a, e, got_immutable))
            }

            (_, _) => got == expected_id,
        }
    }
    pub fn can_expr_be_assigned_to(
        &mut self,
        expected: types::TypeId,
        got: &ir::Expression,
    ) -> bool {
        let got_immutable = self.is_mutable(&got) == Some(false);
        self.can_be_assigned_to(got.ty(), expected, got_immutable)
    }

    pub fn with_scope<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        self.scopes.push(Scope::new());
        let res = f(self);
        self.drop_scope();
        res
    }
    pub fn with_local_scope(&mut self) -> ScopeGuard<'_> {
        self.scopes.push(Scope::new());
        ScopeGuard { tc: self }
    }
    fn drop_scope(&mut self) -> Scope {
        let scope = self.scopes.pop().unwrap();
        for (_, &id) in &scope.bindings {
            self.infer_symbol_type(id);
        }
        scope
    }
    pub fn infer_symbol_type(&mut self, id: SymbolId) {
        let ty = self.symbol_type_id(id);
        let defined_at = self.symbols.get_symbol(id).defined_at();
        let Some(inferred) = self.infer(ty) else {
            self.error(DiagnosticKind::CannotInferType, defined_at);
            return;
        };
        let symbol = self.symbols.get_symbol_mut(id);
        *symbol.ty_mut() = inferred;
    }

    pub fn with_binding_expectation(&mut self, expected: types::TypeId) -> BindingGuard<'_> {
        let previous = self.binding_expectation;
        self.binding_expectation = Some(expected);
        BindingGuard { tc: self, previous }
    }
    pub fn with_this(&mut self, this: types::TypeId) -> ThisGuard<'_> {
        self.this.push(this);
        ThisGuard { tc: self }
    }
    pub(crate) fn this_type(&self) -> Option<types::TypeId> {
        self.this.last().map(|t| *t)
    }

    pub fn with_this_mutability(&mut self, mutable: bool) -> MutableThisGuard<'_> {
        let previous = self.mutable_this;
        self.mutable_this = Some(mutable);
        MutableThisGuard { tc: self, previous }
    }

    pub(super) fn current_scope(&mut self) -> &mut Scope {
        self.scopes.last_mut().unwrap()
    }

    pub fn error(&mut self, kind: DiagnosticKind, loc: Location) {
        self.diagnostics
            .entry(self.current_module)
            .or_default()
            .push(Diagnostic {
                level: DiagnosticLevel::Error,
                loc,
                kind,
            });
    }
    pub fn errors(&mut self, kinds: Vec<DiagnosticKind>, loc: Location) {
        if kinds.is_empty() {
            return;
        };
        let diags = self.diagnostics.entry(self.current_module).or_default();
        for kind in kinds {
            diags.push(Diagnostic {
                level: DiagnosticLevel::Error,
                loc,
                kind,
            });
        }
    }

    pub(crate) fn cancel_diag<F>(&mut self, predicate: F)
    where
        F: Fn(&Diagnostic) -> bool,
    {
        let diagnostics = self.diagnostics.entry(self.current_module).or_default();
        diagnostics.retain(|diag| !predicate(diag));
    }

    pub fn get_symbol_id(&self, name: &str) -> Option<SymbolId> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.lookup(name))
    }

    pub fn lookup(&self, name: &str) -> Option<&dyn Symbol> {
        self.get_symbol_id(name)
            .map(|id| self.symbols.get_symbol(id))
    }

    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut dyn SymbolMut> {
        let id = self.get_symbol_id(name)?;
        Some(self.symbols.get_symbol_mut(id))
    }

    /// Resolve the original symbol behind a type.
    ///
    /// If the given type is a type ref with type arguments, the function
    /// returns the generic definition of the type (ir either a struct, enum or
    /// generic alias).
    pub fn resolve_type_symbol(&self, mut ty: types::TypeId) -> Option<TypeSymbolId> {
        while let types::Type::Ref(r) = self.resolve(ty) {
            ty = r.inner
        }
        self.symbols
            .find_id::<StructSymbolId, _>(|s| s.ty == ty)
            .map(Into::into)
            .or_else(|| {
                self.symbols
                    .find_id::<EnumSymbolId, _>(|s| s.ty == ty)
                    .map(Into::into)
            })
            .or_else(|| {
                self.symbols
                    .find_id::<PrimitiveTypeSymbolId, _>(|s| s.ty == ty)
                    .map(Into::into)
            })
    }

    pub fn is_mutable(&self, expr: &ir::Expression) -> Option<bool> {
        use tine_ir::Expression::*;
        match expr {
            Identifier(i) => Some(self.symbols.is_mutable(i.symbol)),
            Member(m) => match m.object.as_deref() {
                Some(o) => self.is_mutable(o),
                None => self.mutable_this,
            },
            _ => None,
        }
    }

    /// Iterates through all the symbols captured by this expression
    pub fn dependencies<'s>(
        &'s self,
        expr: &'s ir::Expression,
    ) -> Box<dyn Iterator<Item = &'s ir::Identifier> + 's> {
        Box::new(
            expr.walk()
                .filter_map(|child| child.as_expression())
                .filter_map(|child| child.as_identifier())
                .filter(|i| {
                    let symbol = self.symbols.get_symbol(i.symbol);
                    !symbol.defined_at().is_within(expr.loc())
                }),
        )
    }

    /// Given a value whose type implements the given trait, check if an
    /// immutable version of the type also implements the trait (since some
    /// methods might be defined with a mutable receiver).
    pub(super) fn implements_trait(
        &mut self,
        ty: types::TypeId,
        expected_trait: &types::TraitType,
        got_immutable: bool,
    ) -> bool {
        if expected_trait.methods.is_empty() {
            return true;
        }
        if let types::Type::Trait(test) = &self.types.get(ty) {
            if *test == *expected_trait {
                return true;
            }
        }
        let Some(type_symbol) = self.resolve_type_symbol(ty) else {
            return false;
        };
        let mut methods = self.symbol_methods(type_symbol).to_vec();
        methods.retain(|m| self.is_visible((*m).into()));
        if got_immutable {
            methods.retain(|m| !self.symbols.get(*m).is_mutating());
        }

        let got = methods
            .into_iter()
            .map(|m| {
                let symbol = self.symbols.get(m);
                types::TraitMethod {
                    self_type: None,
                    name: symbol.name.clone(),
                    def: symbol.ty,
                }
            })
            .collect::<Vec<_>>();

        expected_trait
            .methods
            .iter()
            .cloned()
            .map(|mut m| {
                if let Some(s) = &m.self_type {
                    let subs = Substitutions::with_initial(&[s.clone()], &[ty]);
                    m.self_type = None;
                    m.def = subs.apply(&mut self.types, m.def);
                }
                m
            })
            .find(|expected| !got.contains(expected))
            .is_none()
    }

    pub(super) fn find_export(&self, module: ModuleId, name: &str) -> Option<SymbolId> {
        self.exports
            .get(&module)
            .and_then(|exports| exports.get(name))
            .copied()
    }

    pub(super) fn module_path(&self) -> &ModulePath {
        self.loader.get_name(self.current_module)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Scope {
    bindings: HashMap<String, SymbolId>,
}

impl Scope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lookup(&self, name: &str) -> Option<SymbolId> {
        self.bindings.get(name).cloned()
    }

    pub fn bind(&mut self, name: String, id: SymbolId) {
        self.bindings.insert(name, id);
    }

    pub fn has(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    pub fn as_bindings(self) -> HashMap<String, SymbolId> {
        self.bindings
    }
}
