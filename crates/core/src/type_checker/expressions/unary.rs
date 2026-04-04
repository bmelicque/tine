use crate::{
    ast,
    ir::{self, root_identifier},
    type_checker::{analysis_context::type_store::TypeStore, TypeChecker},
    types::{self, Type},
    DiagnosticKind,
};

impl TypeChecker<'_> {
    pub fn visit_unary_expression(
        &mut self,
        node: ast::UnaryExpression,
    ) -> Option<ir::UnaryExpression> {
        match node.operator {
            ast::UnaryOperator::Ampersand => self.visit_reference(node),
            ast::UnaryOperator::Bang => self.visit_logical_not_expresion(node),
            ast::UnaryOperator::Minus => self.visit_negate_expresion(node),
            ast::UnaryOperator::Star => self.visit_indirection(node),
        }
    }

    fn visit_indirection(&mut self, node: ast::UnaryExpression) -> Option<ir::UnaryExpression> {
        let Some(operand) = node.operand.and_then(|o| self.visit_expression(*o)) else {
            return None;
        };
        let ty = match self.resolve(operand.ty()) {
            Type::Listener(l) => l.inner,
            Type::Reference(r) => r.target,
            Type::Signal(s) => s.inner,
            Type::Unknown => return None,
            _ => {
                let error = DiagnosticKind::NotDereferenceable {
                    type_name: self.session.display_type(operand.ty()),
                };
                self.error(error, node.loc);
                return None;
            }
        };
        Some(ir::UnaryExpression {
            loc: node.loc,
            operator: node.operator,
            operand: Box::new(operand),
            ty,
        })
    }

    fn visit_reference(&mut self, node: ast::UnaryExpression) -> Option<ir::UnaryExpression> {
        let Some(operand) = node.operand.and_then(|o| self.visit_expression(*o)) else {
            return None;
        };
        if let ir::Expression::Identifier(id) = &operand {
            if let Some(handle) = self.session.get_handle(id.symbol.clone()) {
                handle.read_to_mutable_ref(id.loc);
            }
            if !id.symbol.borrow().is_mutable() {
                let error = DiagnosticKind::RefToConstant { name: id.as_name() };
                self.error(error, node.loc);
            }
        } else if let Some(id) = root_identifier(&operand) {
            if let Some(info) = self.lookup_mut(&id.as_name()) {
                if !info.is_mutable() {
                    let error = DiagnosticKind::AssignmentToConstant { name: id.as_name() };
                    self.error(error, node.loc);
                } else {
                    info.read_to_write(id.loc);
                }
            };
        };

        let ty = self.intern(types::ReferenceType {
            target: operand.ty(),
        });
        Some(ir::UnaryExpression {
            loc: node.loc,
            operator: node.operator,
            operand: Box::new(operand),
            ty,
        })
    }

    fn visit_negate_expresion(
        &mut self,
        node: ast::UnaryExpression,
    ) -> Option<ir::UnaryExpression> {
        let Some(operand) = node.operand.and_then(|o| self.visit_expression(*o)) else {
            return None;
        };
        let operand_type = operand.ty();
        match operand_type {
            TypeStore::INTEGER | TypeStore::FLOAT => Some(ir::UnaryExpression {
                loc: node.loc,
                operator: node.operator,
                operand: Box::new(operand),
                ty: operand_type,
            }),
            TypeStore::UNKNOWN => None,
            _ => {
                let error = DiagnosticKind::ExpectedNumber {
                    got: self.session.display_type(operand_type),
                };
                self.error(error, operand.loc());
                None
            }
        }
    }

    fn visit_logical_not_expresion(
        &mut self,
        node: ast::UnaryExpression,
    ) -> Option<ir::UnaryExpression> {
        let Some(operand) = node.operand.and_then(|o| self.visit_expression(*o)) else {
            return None;
        };
        let operand_type = operand.ty();
        if operand_type != TypeStore::BOOLEAN && operand_type != TypeStore::UNKNOWN {
            let error = DiagnosticKind::ExpectedBool {
                got: self.session.display_type(operand_type),
            };
            self.error(error, operand.loc());
            return None;
        }
        Some(ir::UnaryExpression {
            loc: node.loc,
            operator: node.operator,
            operand: Box::new(operand),
            ty: TypeStore::BOOLEAN,
        })
    }
}
