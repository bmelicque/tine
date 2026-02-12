use crate::{
    ast::{self, utils::root_identifier},
    type_checker::{analysis_context::type_store::TypeStore, TypeChecker},
    types::{ListenerType, ReferenceType, SignalType, Type, TypeId},
    DiagnosticKind,
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
        let expr_type = self.get_operand_type(&node.operand);
        let ty = match self.resolve(expr_type) {
            Type::Listener(l) => l.inner,
            Type::Reference(r) => r.target,
            Type::Signal(s) => s.inner,
            Type::Unknown => TypeStore::UNKNOWN,
            _ => {
                let error = DiagnosticKind::NotDereferenceable {
                    type_name: self.session.display_type(expr_type),
                };
                self.error(error, node.loc);
                TypeStore::UNKNOWN
            }
        };
        self.ctx.save_expression_type(node.loc, ty)
    }

    fn visit_signal_expression(&mut self, node: &ast::UnaryExpression) -> TypeId {
        let inner = self.get_operand_type(&node.operand);
        let ty = self.intern(Type::Signal(SignalType { inner }));
        self.ctx.save_expression_type(node.loc, ty)
    }

    fn visit_listener(&mut self, node: &ast::UnaryExpression) -> TypeId {
        let Some(operand) = &node.operand else {
            return self.ctx.save_expression_type(node.loc, TypeStore::UNKNOWN);
        };
        let (expr_type, deps) = self.with_dependencies(|s| s.visit_expression(operand));
        let count = self.save_reactive_dependencies(&deps, node.loc);
        if count == 0 {
            self.error(DiagnosticKind::NonReactiveExpression, operand.loc());
        }
        self.ctx.add_dependencies(deps);
        if let ast::Expression::Identifier(id) = operand.as_ref() {
            if let Some(info) = self.lookup_mut(&id.as_str()) {
                info.reference(id.loc);
            };
        }

        let ty = self.intern(Type::Listener(ListenerType { inner: expr_type }));
        self.ctx.save_expression_type(node.loc, ty)
    }

    fn visit_reference(&mut self, node: &ast::UnaryExpression) -> TypeId {
        let expr_type = self.get_operand_type(&node.operand);
        let Some(operand) = &node.operand else {
            return self.ctx.save_expression_type(node.loc, TypeStore::UNKNOWN);
        };
        if let ast::Expression::Identifier(id) = &**operand {
            if let Some(info) = self.lookup_mut(&id.as_str()) {
                if !info.is_mutable() {
                    let error = DiagnosticKind::RefToConstant {
                        name: id.as_str().to_string(),
                    };
                    self.error(error, node.loc);
                } else {
                    info.read_to_mutable_ref(id.loc);
                }
            };
        } else if let Some(id) = root_identifier(operand) {
            if let Some(info) = self.lookup_mut(&id.as_str()) {
                if !info.is_mutable() {
                    let error = DiagnosticKind::AssignmentToConstant {
                        name: id.as_str().to_string(),
                    };
                    self.error(error, node.loc);
                } else {
                    info.read_to_write(id.loc);
                }
            };
        };

        let ty = self.intern(Type::Reference(ReferenceType { target: expr_type }));
        self.ctx.save_expression_type(node.loc, ty)
    }

    fn visit_negate_expresion(&mut self, node: &ast::UnaryExpression) -> TypeId {
        let expr_type = self.get_operand_type(&node.operand);
        let is_numeric = expr_type == TypeStore::INTEGER || expr_type == TypeStore::FLOAT;
        if !is_numeric && expr_type != TypeStore::UNKNOWN {
            let error = DiagnosticKind::ExpectedNumber {
                got: self.session.display_type(expr_type),
            };
            // Can safely unwrap() because `None` operand results in `UNKNOWN`
            self.error(error, node.operand.as_ref().unwrap().loc());
            return self.ctx.save_expression_type(node.loc, TypeStore::UNKNOWN);
        }
        self.ctx.save_expression_type(node.loc, TypeStore::INTEGER)
    }

    fn visit_logical_not_expresion(&mut self, node: &ast::UnaryExpression) -> TypeId {
        let expr_type = self.get_operand_type(&node.operand);
        if expr_type != TypeStore::BOOLEAN && expr_type != TypeStore::UNKNOWN {
            let error = DiagnosticKind::ExpectedBool {
                got: self.session.display_type(expr_type),
            };
            // Can safely unwrap() because `None` operand results in `UNKNOWN`
            self.error(error, node.operand.as_ref().unwrap().loc());
            return self.ctx.save_expression_type(node.loc, TypeStore::UNKNOWN);
        }
        self.ctx.save_expression_type(node.loc, TypeStore::BOOLEAN)
    }

    fn get_operand_type(&mut self, operand: &Option<Box<ast::Expression>>) -> TypeId {
        operand
            .as_ref()
            .map(|e| self.visit_expression(&e))
            .unwrap_or(TypeStore::UNKNOWN)
    }
}
