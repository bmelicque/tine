use super::TypeChecker;
use crate::{ast, parser::parser::ParseError, types::Type};

impl TypeChecker {
    pub fn visit_statement(&mut self, node: &ast::Statement) -> Type {
        match node {
            ast::Statement::Assignment(node) => self.visit_assignment(node),
            ast::Statement::Empty => Type::Void,
            ast::Statement::Expression(node) => self.visit_expression(&node.expression),
            ast::Statement::Break(node) => self.visit_break_statement(node),
            ast::Statement::MethodDefinition(node) => self.visit_method_definition(node),
            ast::Statement::Return(node) => self.visit_return_statement(node),
            ast::Statement::TypeAlias(node) => self.visit_type_declaration(node),
            ast::Statement::VariableDeclaration(node) => self.visit_variable_declaration(node),
        }
    }

    fn visit_assignment(&mut self, node: &ast::Assignment) -> Type {
        let value_type = self.visit_expression(&node.value);
        self.visit_assignee(&node.pattern, value_type);

        Type::Void
    }

    fn visit_assignee(&mut self, pattern: &ast::PatternExpression, against: Type) {
        match pattern {
            ast::PatternExpression::Pattern(ref pattern) => {
                self.visit_pattern_assignee(pattern, against)
            }
            ast::PatternExpression::Expression(expr) => match expr {
                ast::Expression::FieldAccess(expr) => {
                    self.visit_expr_assignee(expr, against);
                }
                ast::Expression::TupleIndexing(expr) => {
                    self.visit_expr_assignee(expr, against);
                }
                expr => unreachable!("unexpected expression: {:?}", expr),
            },
        };
    }

    fn visit_pattern_assignee(&mut self, pattern: &ast::Pattern, against: Type) {
        let mut variables = Vec::new();
        self.match_pattern(pattern, against, &mut variables);
        for (name, ty) in variables {
            let Some(info) = self.symbols.lookup(&name) else {
                self.errors.push(ParseError {
                    message: format!("Cannot find name '{}'", name),
                    span: pattern.as_span(),
                });
                continue;
            };
            if info.ty != ty {
                self.errors.push(ParseError {
                    message: format!("Cannot assign type {:?} to {:?}", ty, info.ty),
                    span: pattern.as_span(),
                });
            }
            if !info.mutable {
                self.errors.push(ParseError {
                    message: "Cannot assign to immutable variable".to_string(),
                    span: pattern.as_span(),
                });
            }
        }
    }

    fn visit_expr_assignee(&mut self, expr: &dyn ast::PathExpression, against: Type) {
        let ty = self.visit_expression(expr.base_expression());
        if ty != against {
            self.errors.push(ParseError {
                message: format!("Cannot assign type {:?} to {:?}", against, ty),
                span: expr.as_span(),
            });
        }
        let root = expr.root_expression();
        let ast::Expression::Identifier(root) = root else {
            self.errors.push(ParseError {
                message: "Expected identifier".to_string(),
                span: root.as_span(),
            });
            return;
        };
        let Some(info) = self.symbols.lookup(root.as_str()) else {
            self.errors.push(ParseError {
                message: format!("Cannot find name '{}'", root.as_str()),
                span: root.span,
            });
            return;
        };
        if !info.mutable {
            self.errors.push(ParseError {
                message: "Cannot assign to immutable variable".to_string(),
                span: expr.as_span(),
            });
        }
    }

    fn visit_break_statement(&mut self, node: &ast::BreakStatement) -> Type {
        if let Some(ref value) = node.value {
            self.visit_expression(value);
        }
        Type::Void
    }

    fn visit_method_definition(&mut self, node: &ast::MethodDefinition) -> Type {
        self.symbols.enter_scope();
        let receiver = self.visit_named_type(&node.receiver.ty);
        self.symbols
            .define(node.receiver.name.as_str(), receiver.clone(), false);
        let function = self.visit_function_expression(&node.definition);
        self.symbols.exit_scope();
        let type_name = node.receiver.ty.name.as_str();
        let method_name = node.name.as_str();

        let Type::Named(receiver) = receiver else {
            self.errors.push(ParseError {
                message: format!("Cannot define method on type '{}'", type_name),
                span: node.span,
            });
            return Type::Void;
        };

        if self.type_registry.type_has(type_name, method_name) {
            self.errors.push(ParseError {
                message: format!(
                    "Name '{}' already exists on type {}",
                    method_name, type_name
                ),
                span: node.span,
            });
            return Type::Void;
        }

        self.type_registry
            .define_method(receiver, method_name.into(), function);

        Type::Void
    }

    fn visit_return_statement(&mut self, node: &ast::ReturnStatement) -> Type {
        if let Some(ref value) = node.value {
            self.visit_expression(value);
        }
        Type::Void
    }

    pub fn visit_variable_declaration(&mut self, node: &ast::VariableDeclaration) -> Type {
        let inferred_type = self.visit_expression(&node.value);
        let mutable = node.op == ast::DeclarationOp::Mut;
        let mut variables = Vec::<(String, Type)>::new();
        if node.pattern.is_refutable() {
            self.errors.push(ParseError {
                message: "Irrefutable pattern expected".into(),
                span: node.pattern.as_span(),
            });
        }
        self.match_pattern(&node.pattern, inferred_type, &mut variables);
        for (name, ty) in variables {
            self.symbols.define(&name, ty, mutable);
        }
        Type::Void
    }
}
