use pest::Span;

use super::scopes::TypeRegistry;

use crate::ast;
use crate::parser::parser::ParseError;
use crate::type_checker::analysis_context::{AnalysisContext, SymbolId};
use crate::type_checker::std::dom::node::node;
use crate::types;

pub struct TypeChecker {
    pub errors: Vec<ParseError>,
    pub type_registry: TypeRegistry,
    pub analysis_context: AnalysisContext,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut analysis_context = AnalysisContext::new();
        analysis_context.enter_scope(pest::Span::new("", 0, 0).unwrap());
        let mut type_registry = TypeRegistry::new();
        type_registry.define("Node", node(), None);

        Self {
            errors: Vec::new(),
            type_registry,
            analysis_context,
        }
    }

    pub fn check(&mut self, program: &ast::Program) -> Result<(), Vec<ParseError>> {
        program.statements.iter().for_each(|st| {
            self.visit_statement(st);
        });
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    pub(super) fn resolve_type(&mut self, node: &ast::Type) -> types::Type {
        match node {
            ast::Type::Named(named) => match named.name.as_str() {
                "string" => types::Type::String,
                "number" => types::Type::Number,
                "boolean" => types::Type::Boolean,
                "void" => types::Type::Void,
                id => match self.type_registry.lookup(id) {
                    Some(ty) => ty.clone(),
                    None => {
                        self.errors.push(ParseError {
                            message: format!("Unknown type: {}", id),
                            span: named.span,
                        });
                        types::Type::Unknown
                    }
                },
            },
            _ => panic!("Not implemented yet!"),
        }
    }

    /// If type is Named, unwraps the underlying type, else returns original type
    /// TODO: it should also resolve type arguments
    pub fn unwrap_named_type(&self, ty: &types::Type) -> types::Type {
        match ty {
            types::Type::Named(named) => match named.name.as_str() {
                "string" => types::Type::String,
                "number" => types::Type::Number,
                "boolean" => types::Type::Boolean,
                "void" => types::Type::Void,
                id => self
                    .type_registry
                    .lookup(id)
                    .unwrap_or(types::Type::Unknown),
            },
            _ => ty.clone(),
        }
    }

    pub(super) fn set_type_at<T: Clone + Into<types::Type>>(
        &mut self,
        span: Span<'static>,
        ty: T,
    ) -> T {
        self.analysis_context.types.insert(span, ty.clone().into());
        ty
    }

    pub fn get_type_at(&mut self, span: Span<'static>) -> Option<types::Type> {
        self.analysis_context.types.get(&span).map(|ty| ty.clone())
    }

    pub fn can_be_assigned_to(&self, test: &types::Type, against: &types::Type) -> bool {
        match (test, against) {
            (types::Type::Unknown, _) => true,
            (_, types::Type::Unknown) => true,
            (_, types::Type::Duck(duck)) => self.implements(test, duck),
            (_, _) => test == against,
        }
    }
    fn implements(&self, test: &types::Type, duck: &types::DuckType) -> bool {
        let types::Type::Struct(ref st) = *duck.like else {
            return false;
        };

        let types::Type::Named(name) = test else {
            return false;
        };

        st.fields
            .iter()
            .find(|field| !self.type_registry.type_has(&name.name, &field.name))
            .is_none()
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

    pub fn with_dependencies<F, T>(&mut self, mut predicate: F) -> (T, Vec<SymbolId>)
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
    pub fn save_reactive_dependencies(&mut self, deps: &Vec<SymbolId>, at: Span<'static>) {
        let deps: Vec<SymbolId> = deps
            .into_iter()
            .filter(|dep| self.analysis_context.symbols[**dep].ty.is_reactive())
            .map(|id_ref| *id_ref)
            .collect();
        let len = deps.len();
        if len > 0 {
            self.analysis_context.other_dependencies.insert(at, deps);
        } else {
            self.error(
                "Expected reactive values in listened expression".to_string(),
                at,
            );
        }
    }

    pub fn error(&mut self, message: String, span: Span<'static>) {
        self.errors.push(ParseError { message, span });
    }
}
