use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use tine_ir as ir;
use tine_types::{store::TypeStore, types};

use crate::codegen::{
    expressions::{utils::logical_not, ExpressionResult},
    utils::internal_method_call,
    CodeGenerator,
};

impl CodeGenerator<'_, '_> {
    pub fn handle_binary_expression(&mut self, node: ir::BinaryExpression) -> ExpressionResult {
        let op = match node.op {
            ir::BinaryOperator::Add => swc::BinaryOp::Add,
            ir::BinaryOperator::Div => swc::BinaryOp::Div,
            ir::BinaryOperator::EqEq => return self.handle_eq(node),
            ir::BinaryOperator::Neq => return self.handle_neq(node),
            ir::BinaryOperator::Geq => swc::BinaryOp::GtEq,
            ir::BinaryOperator::Grt => swc::BinaryOp::Gt,
            ir::BinaryOperator::LAnd => swc::BinaryOp::LogicalAnd,
            ir::BinaryOperator::LOr => swc::BinaryOp::LogicalOr,
            ir::BinaryOperator::Leq => swc::BinaryOp::LtEq,
            ir::BinaryOperator::Less => swc::BinaryOp::Lt,
            ir::BinaryOperator::Mod => swc::BinaryOp::Mod,
            ir::BinaryOperator::Mul => swc::BinaryOp::Mul,
            ir::BinaryOperator::Pow => swc::BinaryOp::Exp,
            ir::BinaryOperator::Sub => swc::BinaryOp::Sub,
        };
        let is_int = node.ty == TypeStore::INTEGER;

        let (left, right) = self.handle_binary_operands(node);

        let mut expr = swc::BinExpr {
            span: DUMMY_SP,
            op,
            left: Box::new(left.expr),
            right: Box::new(right.expr),
        };
        if is_int {
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

    fn handle_eq(&mut self, node: ir::BinaryExpression) -> ExpressionResult {
        use types::Type::*;
        match self.types.get(node.ty) {
            Ref(r) if r.inner == TypeStore::ARRAY => self.handle_array_eq(node),
            Tuple(_) => self.handle_array_eq(node),
            _ => self.handle_simple_bin(node, swc::BinaryOp::EqEqEq),
        }
    }

    fn handle_array_eq(&mut self, node: ir::BinaryExpression) -> ExpressionResult {
        let (left_result, right_result) = self.handle_binary_operands(node);
        let mut prelim_stmts = left_result.prelim_stmts;
        prelim_stmts.extend(right_result.prelim_stmts);
        let (left, right) = (left_result.expr.into(), right_result.expr.into());
        let expr = internal_method_call("eqArray", vec![left, right]).into();
        ExpressionResult { prelim_stmts, expr }
    }

    fn handle_neq(&mut self, node: ir::BinaryExpression) -> ExpressionResult {
        use types::Type::*;
        match self.types.get(node.ty) {
            Ref(r) if r.inner == TypeStore::ARRAY => self.handle_array_eq(node).map(logical_not),
            Tuple(_) => self.handle_array_eq(node).map(logical_not),
            _ => self.handle_simple_bin(node, swc::BinaryOp::NotEqEq),
        }
    }

    fn handle_binary_operands(
        &mut self,
        node: ir::BinaryExpression,
    ) -> (ExpressionResult, ExpressionResult) {
        let left = self.handle_expression(*node.left);
        let right = self.handle_expression(*node.right);

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

    fn handle_simple_bin(
        &mut self,
        node: ir::BinaryExpression,
        op: swc::BinaryOp,
    ) -> ExpressionResult {
        let (left, right) = self.handle_binary_operands(node);

        let expr = swc::Expr::Bin(swc::BinExpr {
            span: DUMMY_SP,
            op,
            left: Box::new(left.expr),
            right: Box::new(right.expr),
        });

        ExpressionResult {
            prelim_stmts: vec![left.prelim_stmts, right.prelim_stmts].concat(),
            expr: expr.into(),
        }
    }
}
