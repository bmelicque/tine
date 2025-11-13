use super::TypeChecker;
use crate::{
    ast,
    parser::parser::ParseError,
    type_checker::analysis_context::VariableData,
    types::{FunctionType, Type},
    utils::subspan_from_str,
};

impl TypeChecker {
    pub fn visit_statement(&mut self, node: &ast::Statement) -> Type {
        match node {
            ast::Statement::Assignment(node) => self.visit_assignment(node),
            ast::Statement::Empty => Type::Void,
            ast::Statement::Expression(node) => self.visit_expression(&node.expression),
            ast::Statement::Break(node) => self.visit_break_statement(node),
            ast::Statement::Invalid(_) => Type::Unknown,
            ast::Statement::MethodDefinition(node) => self.visit_method_definition(node),
            ast::Statement::Return(node) => self.visit_return_statement(node),
            ast::Statement::TypeAlias(node) => self.visit_type_declaration(node),
            ast::Statement::VariableDeclaration(node) => self.visit_variable_declaration(node),
        }
    }

    fn visit_assignment(&mut self, node: &ast::Assignment) -> Type {
        let value_type = match &node.value {
            ast::Expression::Empty => Type::Unknown,
            value => self.visit_expression(value),
        };
        self.visit_assignee(&node.pattern, value_type);

        Type::Void
    }

    fn visit_assignee(&mut self, assignee: &ast::Assignee, against: Type) {
        match assignee {
            ast::Assignee::Member(expr) => self.visit_expr_assignee(expr, against),
            ast::Assignee::Indirection(expr) => self.visit_indirect_assignee(expr, against),
            ast::Assignee::Pattern(pat) => self.visit_pattern_assignee(pat, against),
        }
    }

    fn visit_pattern_assignee(&mut self, pattern: &ast::Pattern, against: Type) {
        let mut variables = Vec::new();
        self.match_pattern(pattern, against, &mut variables);
        for (name, ty) in variables {
            let Some(info) = self.analysis_context.lookup_mut(&name) else {
                self.errors.push(ParseError {
                    message: format!("Cannot find name '{}'", name),
                    span: pattern.as_span(),
                });
                continue;
            };
            info.add_write();
            if *info.borrow().ty != ty && !ty.is_unknown() {
                self.errors.push(ParseError {
                    message: format!("Cannot assign type '{}' to type '{}'", ty, info.borrow().ty),
                    span: pattern.as_span(),
                });
            }
            if !info.borrow().mutable {
                self.errors.push(ParseError {
                    message: "Cannot assign to immutable variable".to_string(),
                    span: pattern.as_span(),
                });
            }
        }
    }

    fn visit_expr_assignee(&mut self, expr: &ast::MemberExpression, against: Type) {
        let ty = self.visit_expression(&expr.object);
        if ty != against {
            self.errors.push(ParseError {
                message: format!("Cannot assign type {:?} to {:?}", against, ty),
                span: expr.span,
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
        let Some(info) = self.analysis_context.lookup_mut(root.as_str()) else {
            self.errors.push(ParseError {
                message: format!("Cannot find name '{}'", root.as_str()),
                span: root.span,
            });
            return;
        };
        info.add_write();
        // visit expression at the beginning of the current scope adds a read
        // so we need to remove it here
        info.remove_read();
        if !info.borrow().mutable {
            self.errors.push(ParseError {
                message: "Cannot assign to immutable variable".to_string(),
                span: expr.span,
            });
        }
    }

    fn visit_indirect_assignee(&mut self, node: &ast::IndirectionAssignee, against: Type) {
        let name = node.identifier.as_str();
        let Some(info) = self.analysis_context.lookup_mut(&name) else {
            self.error(format!("Cannot find name '{}'", name), node.identifier.span);
            return;
        };
        info.add_write();
        let ty = info.borrow().ty.clone();
        match *ty {
            Type::Signal(ref ty) => {
                if *ty.inner != against {
                    self.error(
                        format!("Cannot assign type {:?} to {:?}", against, ty),
                        node.span,
                    )
                }
            }
            Type::Listener(ref ty) => {
                if *ty.inner != against {
                    self.error(
                        format!("Cannot assign type {:?} to {:?}", against, ty),
                        node.span,
                    )
                }
            }
            ref ty => {
                self.error(
                    format!("Cannot dereference variable '{}' of type {}", name, ty),
                    node.span,
                );
            }
        }
    }

    fn visit_break_statement(&mut self, node: &ast::BreakStatement) -> Type {
        if let Some(ref value) = node.value {
            self.visit_expression(value);
        }
        Type::Void
    }

    fn visit_method_definition(&mut self, node: &ast::MethodDefinition) -> Type {
        let ((receiver, function), _) = self.with_dependencies(|s| s.visit_method_expression(node));
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

    fn visit_method_expression(&mut self, node: &ast::MethodDefinition) -> (Type, FunctionType) {
        self.with_scope(node.span, |checker| {
            let receiver = checker.visit_named_type(&node.receiver.ty);
            checker.analysis_context.register_symbol(VariableData::pure(
                node.receiver.name.as_str().into(),
                receiver.clone().into(),
                node.receiver.span,
            ));
            let function = checker.visit_function_expression(&node.definition);
            (receiver, function)
        })
    }

    fn visit_return_statement(&mut self, node: &ast::ReturnStatement) -> Type {
        if let Some(ref value) = node.value {
            self.visit_expression(value);
        }
        Type::Void
    }

    pub fn visit_variable_declaration(&mut self, node: &ast::VariableDeclaration) -> Type {
        let (inferred_type, dependencies) =
            self.with_dependencies(|s| s.visit_expression(&node.value));

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
            if self.analysis_context.find_in_current_scope(&name).is_some() {
                let message = format!("variable '{}' already defined in current scope", name);
                let span = subspan_from_str(node.pattern.as_span(), &name).unwrap();
                self.error(message, span);
            } else {
                self.analysis_context.register_symbol(VariableData::new(
                    name.clone(),
                    ty.clone().into(),
                    mutable,
                    node.pattern.as_span(),
                    dependencies.clone(),
                ));
            }
        }
        Type::Void
    }
}
