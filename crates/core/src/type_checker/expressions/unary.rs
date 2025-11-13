use crate::{
    ast::{self, utils::root_identifier},
    parser::parser::ParseError,
    type_checker::TypeChecker,
    types,
};

impl TypeChecker {
    pub fn visit_unary_expression(&mut self, node: &ast::UnaryExpression) -> types::Type {
        match node.operator {
            ast::UnaryOperator::Ampersand => self.visit_reference(node),
            ast::UnaryOperator::At => self.visit_listener(node).into(),
            ast::UnaryOperator::Bang => self.visit_logical_not_expresion(node),
            ast::UnaryOperator::Dollar => self.visit_signal_expression(node),
            ast::UnaryOperator::Minus => self.visit_negate_expresion(node),
            ast::UnaryOperator::Star => self.visit_indirection(node),
        }
    }

    fn visit_indirection(&mut self, node: &ast::UnaryExpression) -> types::Type {
        let expr_type = self.visit_expression(&node.operand);
        match expr_type {
            types::Type::Listener(l) => self.set_type_at(node.span, *l.inner.clone()),
            types::Type::Reference(r) => self.set_type_at(node.span, *r.target.clone()),
            types::Type::Signal(s) => self.set_type_at(node.span, *s.inner.clone()),
            types::Type::Unknown => self.set_type_at(node.span, types::Type::Unknown),
            _ => {
                self.errors.push(ParseError {
                    message: format!("Cannot dereference type {}", expr_type),
                    span: node.span,
                });
                self.set_type_at(node.span, types::Type::Unknown)
            }
        }
    }

    fn visit_signal_expression(&mut self, node: &ast::UnaryExpression) -> types::Type {
        let expr_type = self.visit_expression(&node.operand);
        let ty = types::SignalType {
            inner: Box::new(expr_type),
        };
        self.set_type_at(node.span, ty.into())
    }

    fn visit_listener(&mut self, node: &ast::UnaryExpression) -> types::ListenerType {
        let (expr_type, deps) = self.with_dependencies(|s| s.visit_expression(&node.operand));
        let count = self.save_reactive_dependencies(&deps, node.span);
        if count == 0 {
            self.error(
                "Expected reactive values in listened expression".to_string(),
                node.operand.as_span(),
            );
        }
        self.analysis_context.add_dependencies(deps);
        if let ast::Expression::Identifier(id) = node.operand.as_ref() {
            if let Some(info) = self.analysis_context.lookup_mut(&id.as_str()) {
                info.add_readonly_ref();
                info.remove_read(); // previous visit_expression added a read
            };
        }

        let inner = Box::new(expr_type);
        self.set_type_at(node.span, types::ListenerType { inner })
    }

    fn visit_reference(&mut self, node: &ast::UnaryExpression) -> types::Type {
        let expr_type = self.visit_expression(&node.operand);
        if let ast::Expression::Identifier(id) = node.operand.as_ref() {
            if let Some(info) = self.analysis_context.lookup_mut(&id.as_str()) {
                if !info.is_mutable() {
                    let error_message = format!(
                        "Cannot take mutable reference of immutable variable '{}'",
                        id.as_str()
                    );
                    self.error(error_message, node.span);
                } else {
                    info.add_mutable_ref();
                    info.remove_read(); // previous visit_expression added a read
                }
            };
        } else if let Some(id) = root_identifier(&node.operand) {
            if let Some(info) = self.analysis_context.lookup_mut(&id.as_str()) {
                if !info.is_mutable() {
                    self.error(
                        format!("Cannot assign to immutable variable '{}'", id.as_str()),
                        node.span,
                    );
                } else {
                    info.add_write();
                    info.remove_read(); // previous visit_expression added a read
                }
            };
        };

        let target = Box::new(expr_type);
        self.set_type_at(node.span, types::ReferenceType { target }.into())
    }

    fn visit_negate_expresion(&mut self, node: &ast::UnaryExpression) -> types::Type {
        let expr_type = self.visit_expression(&node.operand);
        if expr_type != types::Type::Number {
            self.errors.push(ParseError {
                message: format!("Cannot negate type {} (number expected)", expr_type),
                span: node.span,
            });
            return self.set_type_at(node.span, types::Type::Unknown);
        }
        self.set_type_at(node.span, types::Type::Number)
    }

    fn visit_logical_not_expresion(&mut self, node: &ast::UnaryExpression) -> types::Type {
        let expr_type = self.visit_expression(&node.operand);
        if expr_type != types::Type::Boolean {
            self.errors.push(ParseError {
                message: format!(
                    "Cannot apply logical not to type {} (boolean expected)",
                    expr_type
                ),
                span: node.span,
            });
            return self.set_type_at(node.span, types::Type::Unknown);
        }
        self.set_type_at(node.span, types::Type::Boolean)
    }
}
