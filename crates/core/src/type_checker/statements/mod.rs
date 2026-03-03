mod assignments;
mod type_definitions;
mod variable_declarations;

use super::TypeChecker;
use crate::{
    ast,
    diagnostics::DiagnosticKind,
    type_checker::analysis_context::{type_store::TypeStore, SymbolData},
    types::TypeId,
    SymbolKind,
};

impl TypeChecker<'_> {
    pub fn visit_statement(&mut self, node: &ast::Statement) -> TypeId {
        match node {
            ast::Statement::Assignment(node) => self.visit_assignment(node),
            ast::Statement::Break(node) => self.visit_break_statement(node),
            ast::Statement::Enum(node) => self.visit_enum_definition(node),
            ast::Statement::Expression(node) => self.visit_expression(&node.expression),
            ast::Statement::Function(node) => self.visit_function_definition(node),
            ast::Statement::Invalid(_) => TypeStore::UNKNOWN,
            ast::Statement::MethodDefinition(node) => self.visit_method_definition(node),
            ast::Statement::Return(node) => self.visit_return_statement(node),
            ast::Statement::StructDefinition(node) => self.visit_struct_definition(node),
            ast::Statement::TypeAlias(node) => self.visit_type_alias(node),
            ast::Statement::VariableDeclaration(node) => self.visit_variable_declaration(node),
        }
    }

    fn visit_assignment(&mut self, node: &ast::Assignment) -> TypeId {
        let value_type = self.visit_expression_option(&node.value);
        if let Some(pattern) = &node.pattern {
            self.visit_assignee(pattern, value_type);
        }

        TypeStore::UNIT
    }

    fn visit_break_statement(&mut self, node: &ast::BreakStatement) -> TypeId {
        if let Some(ref value) = node.value {
            self.visit_expression(value);
        }
        TypeStore::UNIT
    }

    fn visit_function_definition(&mut self, node: &ast::FunctionDefinition) -> TypeId {
        let ast::FunctionDefinition { docs, definition } = node;
        let docs = docs.as_ref().map(|d| d.text.clone());
        let (ty, dependencies) =
            self.with_dependencies(|checker| checker.visit_function_expression(definition));
        let Some(ref name) = definition.name else {
            return TypeStore::UNIT;
        };
        match self.ctx.find_in_current_scope(name.as_str()) {
            Some(symbol) => {
                let error = DiagnosticKind::DuplicateIdentifier {
                    name: name.as_str().into(),
                };
                self.error(error, name.loc);
                symbol
            }
            None => self.ctx.register_symbol(SymbolData {
                name: name.as_str().to_string(),
                ty,
                kind: SymbolKind::Function {
                    param_names: definition
                        .params
                        .iter()
                        .map(|param| param.name.text.clone())
                        .collect(),
                },
                docs,
                defined_at: name.loc,
                dependencies: dependencies.clone(),
                ..Default::default()
            }),
        };

        TypeStore::UNIT
    }

    fn visit_method_definition(&mut self, node: &ast::MethodDefinition) -> TypeId {
        let ((receiver, function), _) = self.with_dependencies(|s| s.visit_method_expression(node));
        let method_name = node.name.as_str();

        if receiver == TypeStore::UNKNOWN {
            return TypeStore::UNIT;
        }

        let field_exists = self.session.types().has_property(receiver, method_name);
        if field_exists {
            let error = DiagnosticKind::DuplicateFieldName {
                name: method_name.to_string(),
            };
            self.error(error, node.loc);
        } else {
            self.session
                .types()
                .define_method(receiver, method_name, function);
        }

        TypeStore::UNIT
    }

    fn visit_method_expression(&mut self, node: &ast::MethodDefinition) -> (TypeId, TypeId) {
        self.with_scope(|checker| {
            let receiver = checker.visit_named_type(&node.receiver.ty);
            checker.ctx.register_symbol(SymbolData {
                name: node.receiver.name.as_str().into(),
                ty: receiver,
                kind: SymbolKind::constant(),
                defined_at: node.receiver.loc,
                ..Default::default()
            });
            let function = checker.visit_function_expression(&node.definition);
            (receiver, function)
        })
    }

    fn visit_return_statement(&mut self, node: &ast::ReturnStatement) -> TypeId {
        if let Some(ref value) = node.value {
            self.visit_expression(value);
        }
        TypeStore::UNIT
    }
}
