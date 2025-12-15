use std::collections::HashMap;

use crate::analyzer::session::Session;
use crate::analyzer::{ModuleId, ModulePath};
use crate::parser::parser::ParseError;
use crate::type_checker::analysis_context::{LocalContext, SymbolRef};
use crate::type_checker::SymbolHandle;
use crate::types::{Type, TypeId};
use crate::Location;

pub struct CheckResult {
    pub symbols: Vec<SymbolHandle>,
    pub exports: Vec<SymbolRef>,
    pub expressions: HashMap<Location, TypeId>,
    pub dependencies: HashMap<Location, Vec<SymbolRef>>,
    pub diagnostics: Vec<ParseError>,
}

pub struct TypeChecker<'sess> {
    current_module: ModuleId,
    pub(crate) session: &'sess Session,
    pub errors: Vec<ParseError>,
    pub ctx: LocalContext,
}

impl TypeChecker<'_> {
    pub fn new<'sess>(session: &'sess Session, id: ModuleId) -> TypeChecker<'sess> {
        TypeChecker {
            current_module: id,
            session,
            errors: vec![],
            ctx: LocalContext::new(),
        }
    }

    pub fn get_file_name(&self) -> ModulePath {
        self.session.read_module(self.current_module).name.clone()
    }

    pub fn check(mut self) -> CheckResult {
        let ast = self.session.get_ast(self.current_module);
        for item in &ast.items {
            self.visit_item(item);
        }

        CheckResult {
            symbols: self.ctx.symbols,
            exports: self.ctx.scopes[0].bindings.clone(),
            expressions: self.ctx.expressions,
            dependencies: self.ctx.other_dependencies,
            diagnostics: self.errors,
        }
    }

    pub fn intern(&self, ty: Type) -> TypeId {
        self.session.intern(ty)
    }
    pub fn intern_unique(&self, ty: Type) -> TypeId {
        self.session.intern_unique(ty)
    }

    pub fn resolve(&self, id: TypeId) -> Type {
        self.session.get_type(id)
    }

    pub fn get_type_at(&mut self, loc: Location) -> Option<TypeId> {
        self.ctx.expressions.get(&loc).map(|ty| *ty)
    }

    pub fn can_be_assigned_to(&self, test_id: TypeId, against: TypeId) -> bool {
        let test = self.ctx.type_store.get(test_id);
        let against = self.ctx.type_store.get(against);
        match (test, against) {
            (Type::Unknown, _) => true,
            (_, Type::Unknown) => true,
            (_, Type::Duck(duck)) => self.ctx.type_store.implements(test_id, duck),
            (_, _) => test == against,
        }
    }

    pub fn with_scope<F, T>(&mut self, mut predicate: F) -> T
    where
        F: FnMut(&mut Self) -> T,
    {
        self.ctx.enter_scope();
        let res = predicate(self);
        let scope = self.ctx.pop_scope();
        self.ctx.add_dependencies(scope.captured());
        res
    }

    /// Execute the given predicate while registering outer dependencies (=enclosed variables)
    pub fn with_dependencies<F, T>(&mut self, mut predicate: F) -> (T, Vec<SymbolRef>)
    where
        F: FnMut(&mut Self) -> T,
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

    pub fn error(&mut self, message: String, loc: Location) {
        self.errors.push(ParseError { message, loc });
    }
}
