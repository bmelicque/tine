use crate::{
    ast::{self, utils::root_identifier},
    type_checker::{analysis_context::type_store::TypeStore, TypeChecker},
    types::{ListenerType, ReferenceType, SignalType, Type, TypeId},
};

impl TypeChecker<'_> {
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
                self.error(format!("Cannot dereference type {}", ty), node.loc);
                TypeStore::UNKNOWN
            }
        };
        self.ctx.save_expression_type(node.loc, ty)
    }

    fn visit_signal_expression(&mut self, node: &ast::UnaryExpression) -> TypeId {
        let inner = self.visit_expression(&node.operand);
        let ty = self.intern(Type::Signal(SignalType { inner }));
        self.ctx.save_expression_type(node.loc, ty)
    }

    fn visit_listener(&mut self, node: &ast::UnaryExpression) -> TypeId {
        let (expr_type, deps) = self.with_dependencies(|s| s.visit_expression(&node.operand));
        let count = self.save_reactive_dependencies(&deps, node.loc);
        if count == 0 {
            self.error(
                "Expected reactive values in listened expression".to_string(),
                node.operand.loc(),
            );
        }
        self.ctx.add_dependencies(deps);
        if let ast::Expression::Identifier(id) = node.operand.as_ref() {
            if let Some(info) = self.lookup_mut(&id.as_str()) {
                info.reference(id.loc);
            };
        }

        let ty = self.intern(Type::Listener(ListenerType { inner: expr_type }));
        self.ctx.save_expression_type(node.loc, ty)
    }

    fn visit_reference(&mut self, node: &ast::UnaryExpression) -> TypeId {
        let expr_type = self.visit_expression(&node.operand);
        if let ast::Expression::Identifier(id) = node.operand.as_ref() {
            if let Some(info) = self.lookup_mut(&id.as_str()) {
                if !info.is_mutable() {
                    let error_message = format!(
                        "Cannot take mutable reference of immutable variable '{}'",
                        id.as_str()
                    );
                    self.error(error_message, node.loc);
                } else {
                    info.read_to_mutable_ref(id.loc);
                }
            };
        } else if let Some(id) = root_identifier(&node.operand) {
            if let Some(info) = self.lookup_mut(&id.as_str()) {
                if !info.is_mutable() {
                    self.error(
                        format!("Cannot assign to immutable variable '{}'", id.as_str()),
                        node.loc,
                    );
                } else {
                    info.read_to_write(id.loc);
                }
            };
        };

        let ty = self.intern(Type::Reference(ReferenceType { target: expr_type }));
        self.ctx.save_expression_type(node.loc, ty)
    }

    fn visit_negate_expresion(&mut self, node: &ast::UnaryExpression) -> TypeId {
        let expr_type = match node.operand.as_ref() {
            ast::Expression::Empty => TypeStore::UNKNOWN,
            operand => self.visit_expression(operand),
        };
        if expr_type != TypeStore::INTEGER && expr_type != TypeStore::UNKNOWN {
            self.error("expected int".into(), node.operand.loc());
            return self.ctx.save_expression_type(node.loc, TypeStore::UNKNOWN);
        }
        self.ctx.save_expression_type(node.loc, TypeStore::INTEGER)
    }

    fn visit_logical_not_expresion(&mut self, node: &ast::UnaryExpression) -> TypeId {
        let expr_type = match node.operand.as_ref() {
            ast::Expression::Empty => TypeStore::UNKNOWN,
            operand => self.visit_expression(operand),
        };
        if expr_type != TypeStore::BOOLEAN && expr_type != TypeStore::UNKNOWN {
            self.error("expected boolean".into(), node.operand.loc());
            return self.ctx.save_expression_type(node.loc, TypeStore::UNKNOWN);
        }
        self.ctx.save_expression_type(node.loc, TypeStore::BOOLEAN)
    }
}
