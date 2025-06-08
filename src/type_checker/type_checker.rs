use super::scopes::{SymbolTable, TypeRegistry};

use crate::ast;
use crate::parser::parser::ParseError;
use crate::types;

pub struct TypeChecker {
    pub errors: Vec<ParseError>,
    pub symbols: SymbolTable,
    pub type_registry: TypeRegistry,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut symbols = SymbolTable::default();
        symbols.enter_scope();
        Self {
            errors: Vec::new(),
            symbols,
            type_registry: TypeRegistry::new(),
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
}
