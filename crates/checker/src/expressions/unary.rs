use tine_ast as ast;
use tine_common::{diagnostics::DiagnosticKind, locations::Locatable};
use tine_ir::{self as ir, Typed};
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
        let ty = match self.try_deref_type(operand.ty()) {
            Ok(ty) => ty?,
            Err(e) => {
                self.error(e, node.loc);
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
    pub fn try_deref_type(
        &mut self,
        ty: types::TypeId,
    ) -> Result<Option<types::TypeId>, DiagnosticKind> {
        use types::Type::*;
        match self.resolve(ty) {
            Listener(l) => Ok(Some(l.inner)),
            Signal(s) => Ok(Some(s.inner)),
            Unknown => Ok(None),
            _ => {
                let type_name = self.types.display(ty);
                Err(DiagnosticKind::NotDereferenceable { type_name })
            }
        }
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
