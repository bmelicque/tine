use std::collections::HashMap;

use crate::common::module_path::{ModuleId, ModulePath};
use crate::diagnostics::{Diagnostic, DiagnosticKind, DiagnosticLevel};
use crate::type_checker::loader::{CheckerLoader, LoadedModule, MockLoader, ModuleLoader};
use crate::type_checker::symbols::*;
use crate::type_checker::type_store::TypeStore;
use crate::types::{self, Type, TypeId};
use crate::{ir, Location, ProjectParser, Source};

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
    let mut tc = TypeChecker::with_loader(Box::new(loader));
    for module in sorted_modules {
        tc.check_module(module);
    }
    let r = tc.results();
    CheckProjectResult {
        names,
        ids,
        sources: project.sources,
        ir: r.ir,
        types: r.types,
        symbols: r.symbols,
        diagnostics: r.diagnostics,
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

            loader: Box::new(MockLoader),

            ir: HashMap::new(),
            exports: HashMap::new(),
            diagnostics: HashMap::new(),
        };
        tc.init_builtins();
        tc
    }

    pub fn with_loader(loader: Box<dyn ModuleLoader>) -> Self {
        let mut tc = Self::new();
        tc.loader = loader;
        tc
    }

    pub fn check_module(&mut self, module_id: ModuleId) {
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
        let scope = self.scopes.pop().unwrap();

        self.ir.insert(module_id, program);
        self.exports.insert(module_id, scope.as_bindings());
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

    pub fn intern(&mut self, ty: impl Into<Type>) -> TypeId {
        self.types.add(ty.into())
    }
    pub fn intern_unique(&mut self, ty: impl Into<Type>) -> TypeId {
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

    pub fn resolve(&self, id: TypeId) -> Type {
        self.types.get(id).clone()
    }

    pub fn symbol_type_id<S>(&self, symbol: S) -> TypeId
    where
        S: Into<SymbolId>,
    {
        let symbol = symbol.into();
        self.symbols.get_symbol(symbol).ty()
    }

    pub fn symbol_type<S>(&self, symbol: S) -> Type
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

    pub fn can_be_assigned_to(&self, got: TypeId, expected: TypeId) -> bool {
        self.types.can_assign_to(got, expected)
    }

    pub fn with_scope<F, T>(&mut self, predicate: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        self.scopes.push(Scope::new());
        let res = predicate(self);
        self.scopes.pop();
        res
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

    /// Resolve the original symbol behind a type.
    ///
    /// If the given type is a type ref with type arguments, the function
    /// returns the generic definition of the type (ir either a struct, enum or
    /// generic alias).
    pub fn resolve_type_symbol(&self, mut ty: TypeId) -> Option<TypeSymbolId> {
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
    }

    pub fn is_mutable(&self, expr: &ir::Expression) -> Option<bool> {
        use ir::Expression::*;
        match expr {
            Identifier(i) => Some(self.symbols.is_mutable(i.symbol)),
            Member(m) => self.is_mutable(&m.object),
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

        let value_methods = self
            .symbol_methods(type_symbol)
            .into_iter()
            .map(|m| {
                let name = self.symbol_name(*m).to_string();
                let def = self.symbol_type_id(*m);
                (types::TraitMethod { name, def }, m)
            })
            .collect::<Vec<_>>();
        for trait_method in &trait_.methods {
            let (_, value_method) = value_methods.iter().find(|m| m.0 == *trait_method).unwrap();
            if self
                .symbols
                .get::<MethodSymbolId>(**value_method)
                .is_mutating()
            {
                return false;
            }
        }
        true
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
        Self {
            bindings: HashMap::new(),
        }
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
