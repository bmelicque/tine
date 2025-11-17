use crate::{
    ast::{self, utils::root_identifier},
    type_checker::{analysis_context::type_store::TypeStore, TypeChecker},
    types::{ListenerType, ReferenceType, SignalType, Type, TypeId},
};

impl TypeChecker {
    pub fn visit_unary_expression(&mut self, node: &ast::UnaryExpression) -> TypeId {
        match node.operator {
            ast::UnaryOperator::Ampersand => self.visit_reference(node),
            ast::UnaryOperator::At => self.visit_listener(node).into(),
            ast::UnaryOperator::Bang => self.visit_logical_not_expresion(node),
            ast::UnaryOperator::Dollar => self.visit_signal_expression(node),
            ast::UnaryOperator::Minus => self.visit_negate_expresion(node),
            ast::UnaryOperator::Star => self.visit_indirection(node),
        }
    }

    fn visit_indirection(&mut self, node: &ast::UnaryExpression) -> TypeId {
        let expr_type = self.visit_expression(&node.operand);
        let ty = match self.resolve(expr_type) {
            Type::Listener(l) => l.inner,
            Type::Reference(r) => r.target,
            Type::Signal(s) => s.inner,
            Type::Unknown => TypeStore::UNKNOWN,
            ty => {
                self.error(format!("Cannot dereference type {}", *ty), node.span);
                TypeStore::UNKNOWN
            }
        };
        self.analysis_context.save_expression_type(node.span, ty)
    }

    fn visit_signal_expression(&mut self, node: &ast::UnaryExpression) -> TypeId {
        let inner = self.visit_expression(&node.operand);
        let ty = self
            .analysis_context
            .type_store
            .add(Type::Signal(SignalType { inner }));
        self.analysis_context.save_expression_type(node.span, ty)
    }

    fn visit_listener(&mut self, node: &ast::UnaryExpression) -> TypeId {
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

        let ty = self
            .analysis_context
            .type_store
            .add(Type::Listener(ListenerType { inner: expr_type }));
        self.analysis_context.save_expression_type(node.span, ty)
    }

    fn visit_reference(&mut self, node: &ast::UnaryExpression) -> TypeId {
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

        let ty = self
            .analysis_context
            .type_store
            .add(Type::Reference(ReferenceType { target: expr_type }));
        self.analysis_context.save_expression_type(node.span, ty)
    }

    fn visit_negate_expresion(&mut self, node: &ast::UnaryExpression) -> TypeId {
        let expr_type = match node.operand.as_ref() {
            ast::Expression::Empty => TypeStore::UNKNOWN,
            operand => self.visit_expression(operand),
        };
        if expr_type != TypeStore::NUMBER && expr_type != TypeStore::UNKNOWN {
            self.error("expected number".into(), node.operand.as_span());
            return self
                .analysis_context
                .save_expression_type(node.span, TypeStore::UNKNOWN);
        }
        self.analysis_context
            .save_expression_type(node.span, TypeStore::NUMBER)
    }

    fn visit_logical_not_expresion(&mut self, node: &ast::UnaryExpression) -> TypeId {
        let expr_type = match node.operand.as_ref() {
            ast::Expression::Empty => TypeStore::UNKNOWN,
            operand => self.visit_expression(operand),
        };
        if expr_type != TypeStore::BOOLEAN && expr_type != TypeStore::UNKNOWN {
            self.error("expected boolean".into(), node.operand.as_span());
            return self
                .analysis_context
                .save_expression_type(node.span, TypeStore::UNKNOWN);
        }
        self.analysis_context
            .save_expression_type(node.span, TypeStore::BOOLEAN)
    }
}
