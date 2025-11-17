use std::cell::RefCell;
use std::rc::Rc;

use pest::Span;
use swc_common::FileName;

use crate::analyzer;
use crate::parser::parser::ParseError;
use crate::type_checker::analysis_context::{AnalysisContext, ModuleMetadata, VariableRef};
use crate::types::{Type, TypeId};

pub struct CheckResult {
    pub metadata: ModuleMetadata,
    pub errors: Vec<ParseError>,
}

pub struct TypeChecker {
    file_name: Option<Rc<FileName>>,
    project_modules: Vec<Rc<RefCell<analyzer::Module>>>,
    pub errors: Vec<ParseError>,
    pub analysis_context: AnalysisContext,
}

impl TypeChecker {
    pub fn new(external: Vec<Rc<RefCell<analyzer::Module>>>) -> Self {
        let mut analysis_context = AnalysisContext::new();
        analysis_context.enter_scope(pest::Span::new("", 0, 0).unwrap());

        Self {
            file_name: None,
            project_modules: external,
            errors: Vec::new(),
            analysis_context,
        }
    }

    pub fn get_file_name(&self) -> Option<Rc<FileName>> {
        self.file_name.clone()
    }

    pub fn check(mut self, module: &analyzer::Module) -> CheckResult {
        self.file_name = Some(module.name.clone());
        module
            .ast
            .items
            .iter()
            .for_each(|item| self.visit_item(item));
        CheckResult {
            metadata: (&self.analysis_context).into(),
            errors: self.errors,
        }
    }

    pub fn resolve(&self, id: TypeId) -> &Type {
        self.analysis_context.type_store.get(id)
    }

    pub fn get_type_at(&mut self, span: Span<'static>) -> Option<TypeId> {
        self.analysis_context.expressions.get(&span).map(|ty| *ty)
    }

    pub fn can_be_assigned_to(&self, test_id: TypeId, against: TypeId) -> bool {
        let test = self.analysis_context.type_store.get(test_id);
        let against = self.analysis_context.type_store.get(against);
        match (test, against) {
            (Type::Unknown, _) => true,
            (_, Type::Unknown) => true,
            (_, Type::Duck(duck)) => self.analysis_context.type_store.implements(test_id, duck),
            (_, _) => test == against,
        }
    }

    pub fn with_scope<F, T>(&mut self, scope_span: Span<'static>, mut predicate: F) -> T
    where
        F: FnMut(&mut Self) -> T,
    {
        self.analysis_context.enter_scope(scope_span);
        let res = predicate(self);
        let scope = self.analysis_context.exit_scope();
        let captured = scope.captured().into_iter().collect();
        self.analysis_context.add_dependencies(captured);
        res
    }

    /// Execute the given predicate while registering outer dependencies (=enclosed variables)
    pub fn with_dependencies<F, T>(&mut self, mut predicate: F) -> (T, Vec<VariableRef>)
    where
        F: FnMut(&mut Self) -> T,
    {
        let memo = self
            .analysis_context
            .current_declaration_dependencies
            .clone();
        self.analysis_context.current_declaration_dependencies = Some(vec![]);
        let res = predicate(self);
        let dependencies = self
            .analysis_context
            .current_declaration_dependencies
            .clone()
            .unwrap();
        self.analysis_context.current_declaration_dependencies = memo;
        (res, dependencies)
    }

    /// Returns how many dependencies were actually reactive
    pub fn save_reactive_dependencies(
        &mut self,
        deps: &Vec<VariableRef>,
        at: Span<'static>,
    ) -> usize {
        let deps: Vec<VariableRef> = deps
            .into_iter()
            .filter(|dep| self.resolve(dep.borrow().ty).is_reactive())
            .cloned()
            .collect();
        let len = deps.len();
        if len > 0 {
            self.analysis_context.other_dependencies.insert(at, deps);
        }
        len
    }

    pub fn error(&mut self, message: String, span: Span<'static>) {
        self.errors.push(ParseError { message, span });
    }

    pub(super) fn get_module(&self, name: &FileName) -> Option<&Rc<RefCell<analyzer::Module>>> {
        self.project_modules
            .iter()
            .find(|m| *m.borrow().name == *name)
    }
}
