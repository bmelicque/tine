use tine_ast as ast;
use tine_common::diagnostics::DiagnosticKind;
use tine_ir as ir;
use tine_types::{store::TypeStore, types};

use crate::TypeChecker;

impl TypeChecker {
    pub fn visit_unary_expression(
        &mut self,
        node: ast::UnaryExpression,
    ) -> Option<ir::UnaryExpression> {
        match node.operator {
            ast::UnaryOperator::Bang => self.visit_logical_not_expresion(node),
            ast::UnaryOperator::Minus => self.visit_negate_expresion(node),
            ast::UnaryOperator::Star => self.visit_indirection(node),

            ast::UnaryOperator::Mut => panic!(),
        }
    }

    fn visit_indirection(&mut self, node: ast::UnaryExpression) -> Option<ir::UnaryExpression> {
        let Some(operand) = node.operand.and_then(|o| self.visit_expression(*o)) else {
            return None;
        };
        use types::Type::*;
        let ty = match self.resolve(operand.ty()) {
            Listener(l) => l.inner,
            Signal(s) => s.inner,
            Unknown => return None,
            _ => {
                let error = DiagnosticKind::NotDereferenceable {
                    type_name: self.types.display(operand.ty()),
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
                    got: self.types.display(operand_type),
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
                got: self.types.display(operand_type),
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
