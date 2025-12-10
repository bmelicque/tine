use crate::{
    ast,
    locations::Span,
    type_checker::{analysis_context::type_store::TypeStore, TypeChecker},
    types::TypeId,
};

impl TypeChecker {
    pub fn visit_binary_expression(&mut self, node: &ast::BinaryExpression) -> TypeId {
        let left_type = self.visit_expression(&node.left);
        let right_type = self.visit_expression(&node.right);

        match node.operator {
            ast::BinaryOperator::Add
            | ast::BinaryOperator::Sub
            | ast::BinaryOperator::Mul
            | ast::BinaryOperator::Div
            | ast::BinaryOperator::Mod
            | ast::BinaryOperator::Pow
            | ast::BinaryOperator::Geq
            | ast::BinaryOperator::Grt
            | ast::BinaryOperator::Leq
            | ast::BinaryOperator::Less => {
                if left_type != TypeStore::UNKNOWN && left_type != TypeStore::NUMBER {
                    self.push_binary_error(node.operator, left_type, node.span);
                };
                if right_type != TypeStore::UNKNOWN && right_type != TypeStore::NUMBER {
                    self.push_binary_error(node.operator, right_type, node.span);
                };
            }
            ast::BinaryOperator::EqEq | ast::BinaryOperator::Neq => {
                let allow_comparison = left_type == right_type
                    || left_type == TypeStore::UNKNOWN
                    || right_type == TypeStore::UNKNOWN;
                if !allow_comparison {
                    let error = format!(
                        "Types '{}' and '{}' cannot be compared",
                        left_type, right_type
                    );
                    self.error(error, node.span);
                }
            }
            ast::BinaryOperator::LAnd | ast::BinaryOperator::LOr => {
                if left_type != TypeStore::UNKNOWN && left_type != TypeStore::BOOLEAN {
                    self.push_binary_error(node.operator, left_type, node.span);
                };
                if right_type != TypeStore::UNKNOWN && right_type != TypeStore::BOOLEAN {
                    self.push_binary_error(node.operator, right_type, node.span);
                };
            }
        };

        self.analysis_context
            .save_expression_type(node.span, get_binary_expression_type(node.operator))
    }

    fn push_binary_error(&mut self, op: ast::BinaryOperator, ty: TypeId, span: Span) {
        let ty = self.analysis_context.type_store.get(ty);
        self.error(
            format!("Operator '{}' cannot be applied to type '{}'", op, ty),
            span,
        )
    }
}

fn get_binary_expression_type(op: ast::BinaryOperator) -> TypeId {
    match op {
        ast::BinaryOperator::Add
        | ast::BinaryOperator::Sub
        | ast::BinaryOperator::Mul
        | ast::BinaryOperator::Div
        | ast::BinaryOperator::Mod
        | ast::BinaryOperator::Pow => TypeStore::NUMBER,
        ast::BinaryOperator::EqEq
        | ast::BinaryOperator::Geq
        | ast::BinaryOperator::Grt
        | ast::BinaryOperator::LAnd
        | ast::BinaryOperator::Leq
        | ast::BinaryOperator::Less
        | ast::BinaryOperator::LOr
        | ast::BinaryOperator::Neq => TypeStore::BOOLEAN,
    }
}
