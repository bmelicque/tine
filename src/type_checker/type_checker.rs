use pest::Span;

use super::scopes::TypeRegistry;

use crate::ast;
use crate::parser::parser::ParseError;
use crate::type_checker::analysis_context::AnalysisContext;
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
        analysis_context.enter_scope();
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
}
