use swc_common::DUMMY_SP;
use tine_core::{ir, TypeStore};

use crate::codegen::{expressions::ExpressionResult, CodeGenerator};

use swc_ecma_ast as swc;

impl CodeGenerator<'_> {
    pub fn handle_binary_expression(&mut self, node: &ir::BinaryExpression) -> ExpressionResult {
        let (left, right) = self.handle_binary_operands(node);

        let op = match node.op {
            ir::BinaryOperator::Add => swc::BinaryOp::Add,
            ir::BinaryOperator::Div => swc::BinaryOp::Div,
            ir::BinaryOperator::EqEq => swc::BinaryOp::EqEqEq,
            ir::BinaryOperator::Geq => swc::BinaryOp::GtEq,
            ir::BinaryOperator::Grt => swc::BinaryOp::Gt,
            ir::BinaryOperator::LAnd => swc::BinaryOp::LogicalAnd,
            ir::BinaryOperator::LOr => swc::BinaryOp::LogicalOr,
            ir::BinaryOperator::Leq => swc::BinaryOp::LtEq,
            ir::BinaryOperator::Less => swc::BinaryOp::Lt,
            ir::BinaryOperator::Mod => swc::BinaryOp::Mod,
            ir::BinaryOperator::Mul => swc::BinaryOp::Mul,
            ir::BinaryOperator::Neq => swc::BinaryOp::NotEqEq,
            ir::BinaryOperator::Pow => swc::BinaryOp::Exp,
            ir::BinaryOperator::Sub => swc::BinaryOp::Sub,
        };

        let mut expr = swc::BinExpr {
            span: DUMMY_SP,
            op,
            left: Box::new(left.expr),
            right: Box::new(right.expr),
        };
        if node.ty == TypeStore::INTEGER {
            expr = swc::BinExpr {
                span: DUMMY_SP,
                op: swc::BinaryOp::BitOr,
                left: Box::new(expr.into()),
                right: Box::new(swc::Expr::Lit(swc::Lit::Num(swc::Number {
                    span: DUMMY_SP,
                    value: 0.,
                    raw: None,
                }))),
            };
        }

        ExpressionResult {
            prelim_stmts: vec![left.prelim_stmts, right.prelim_stmts].concat(),
            expr: expr.into(),
        }
    }

    fn handle_binary_operands(
        &mut self,
        node: &ir::BinaryExpression,
    ) -> (ExpressionResult, ExpressionResult) {
        let left = self.handle_expression(&node.left);
        let right = self.handle_expression(&node.right);

        match (left.prelim_stmts.is_empty(), right.prelim_stmts.is_empty()) {
            (false, false) | (true, true) => (left, right),
            (false, true) => {
                let right = self.extract_expression(right.expr);
                (left, right)
            }
            (true, false) => {
                let left = self.extract_expression(left.expr);
                (left, right)
            }
        }
    }
}
