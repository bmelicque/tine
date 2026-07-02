use std::collections::HashMap;

use crate::analyzer::ModuleId;
use crate::diagnostics::{Diagnostic, DiagnosticKind, DiagnosticLevel};
use crate::type_checker::symbols::*;
use crate::type_checker::type_store::TypeStore;
use crate::types::{self, Type, TypeId};
use crate::{ast, ir, Location, ModulePath};

#[derive(Debug, Default)]
pub struct CheckResult {
    pub ir: ir::Program,
    pub exports: HashMap<String, SymbolId>,
    pub diagnostics: Vec<Diagnostic>,
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
    pub diagnostics: Vec<Diagnostic>,

    pub(super) loader: Box<dyn ModuleLoader>,

    ir: HashMap<ModuleId, ir::Program>,
    exports: HashMap<ModuleId, HashMap<String, SymbolId>>,
}

impl TypeChecker {
    pub fn new() -> TypeChecker {
        TypeChecker {
            current_module: 0,
            types: TypeStore::new(),
            symbols: SymbolTable::default(),
            scopes: vec![],
            diagnostics: vec![],

            loader: Box::new(MockLoader::new()),

            ir: HashMap::new(),
            exports: HashMap::new(),
        }
    }

    pub fn set_loader(&mut self, loader: Box<dyn ModuleLoader>) {
        self.loader = loader;
    }

    pub fn check_module(mut self, module: ast::Program) -> CheckResult {
        self.scopes.push(Scope::new());
        let program = ir::Program {
            statements: module
                .items
                .iter()
                .flat_map(|i| self.visit_item(i.clone()))
                .collect(),
        };
        let scope = self.scopes.pop().unwrap();

        CheckResult {
            ir: program,
            exports: scope.as_bindings(),
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
            TypeSymbolId::Struct(s) => &self.symbols.get(s).methods,
        }
    }

    pub fn symbol_body<'s, S>(&'s self, symbol: S) -> &'s [MethodSymbolId]
    where
        S: Into<TypeSymbolId>,
    {
        let symbol = symbol.into();
        match symbol {
            TypeSymbolId::Enum(s) => &self.symbols.get(s).methods,
            TypeSymbolId::Struct(s) => &self.symbols.get(s).methods,
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
        self.diagnostics.push(Diagnostic {
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

pub(super) trait ModuleLoader {
    fn find_id(&self, name: &ModulePath) -> Option<ModuleId>;
    fn get_name(&self, module: ModuleId) -> &ModulePath;
    fn module(&self, id: ModuleId) -> ast::Program;
}

struct MockLoader(ModulePath);
impl MockLoader {
    fn new() -> Self {
        Self(ModulePath::Virtual("".to_string()))
    }
}
impl ModuleLoader for MockLoader {
    fn find_id(&self, _name: &ModulePath) -> Option<ModuleId> {
        None
    }
    fn get_name(&self, _module: ModuleId) -> &ModulePath {
        &self.0
    }
    fn module(&self, _id: ModuleId) -> ast::Program {
        ast::Program {
            loc: Location::dummy(),
            items: vec![],
        }
    }
}
