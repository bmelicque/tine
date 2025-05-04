use super::TypeChecker;
use crate::{ast, parser::parser::ParseError, types::Type};

impl TypeChecker {
    pub fn visit_statement(&mut self, node: &ast::Statement) -> Type {
        match node {
            ast::Statement::Assignment(node) => self.visit_assignment(node),
            ast::Statement::Block(node) => self.visit_block_statement(node),
            ast::Statement::Empty => Type::Void,
            ast::Statement::Expression(node) => self.visit_expression(&node.expression),
            ast::Statement::Return(node) => self.visit_return_statement(node),
            ast::Statement::TypeAlias(node) => self.visit_type_declaration(node),
            ast::Statement::VariableDeclaration(node) => self.visit_variable_declaration(node),
        }
    }

    fn visit_assignment(&mut self, node: &ast::Assignment) -> Type {
        let value_type = self.visit_expression(&node.value);
        let name = &node.name;

        let Some(info) = self.symbols.lookup(&name) else {
            self.errors.push(ParseError {
                message: format!("Cannot find name '{}'", name),
                span: node.span,
            });
            return Type::Void;
        };

        if !info.mutable {
            self.errors.push(ParseError {
                message: "Cannot assign to immutable variable".to_string(),
                span: node.span,
            });
        }
        if info.ty != value_type {
            self.errors.push(ParseError {
                message: format!("Cannot assign type {:?} to {:?}", value_type, info.ty),
                span: node.span,
            });
        }

        Type::Void
    }

    pub fn visit_block_statement(&mut self, node: &ast::BlockStatement) -> Type {
        // TODO: handle diverging statemnts (return, break, continue)
        for stmt in node.statements.iter() {
            self.visit_statement(&stmt);
        }
        Type::Void
    }

    fn visit_return_statement(&mut self, node: &ast::ReturnStatement) -> Type {
        if let Some(ref value) = node.value {
            self.visit_expression(value);
        }
        Type::Void
    }

    fn visit_variable_declaration(&mut self, node: &ast::VariableDeclaration) -> Type {
        let inferred_type = self.visit_expression(&node.value);
        let mutable = node.op == ast::DeclarationOp::Mut;
        self.symbols.define(&node.name, inferred_type, mutable);
        Type::Void
    }
}
