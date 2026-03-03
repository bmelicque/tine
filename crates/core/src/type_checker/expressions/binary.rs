use crate::{
    ast,
    diagnostics::DiagnosticKind,
    type_checker::{analysis_context::type_store::TypeStore, TypeChecker},
    types::TypeId,
    Location,
};

impl TypeChecker<'_> {
    pub fn visit_binary_expression(&mut self, node: &ast::BinaryExpression) -> TypeId {
        let left_type = node
            .left
            .as_ref()
            .map(|e| self.visit_expression(&e))
            .unwrap_or(TypeStore::UNKNOWN);
        let right_type = node
            .right
            .as_ref()
            .map(|e| self.visit_expression(&e))
            .unwrap_or(TypeStore::UNKNOWN);

        match node.operator {
            ast::BinaryOperator::Add => {
                let left_is_ok = left_type == TypeStore::INTEGER
                    || left_type == TypeStore::FLOAT
                    || left_type == TypeStore::STRING;
                let right_is_ok = right_type == TypeStore::INTEGER
                    || right_type == TypeStore::FLOAT
                    || right_type == TypeStore::STRING;
                if !left_is_ok && left_type != TypeStore::UNKNOWN {
                    // Can unwrap safely because `None` results in `UNKNOWN`
                    self.push_binary_error(
                        node.operator,
                        left_type,
                        node.left.as_ref().unwrap().loc(),
                    );
                };
                if !right_is_ok && right_type != TypeStore::UNKNOWN {
                    // Can unwrap safely because `None` results in `UNKNOWN`
                    self.push_binary_error(
                        node.operator,
                        right_type,
                        node.right.as_ref().unwrap().loc(),
                    );
                };
                if left_is_ok && right_is_ok && left_type != right_type {
                    let error = DiagnosticKind::MismatchedTypes {
                        left_name: self.session.display_type(left_type),
                        right_name: self.session.display_type(right_type),
                    };
                    self.error(error, node.loc);
                };
            }
            ast::BinaryOperator::Sub
            | ast::BinaryOperator::Mul
            | ast::BinaryOperator::Div
            | ast::BinaryOperator::Mod
            | ast::BinaryOperator::Pow
            | ast::BinaryOperator::Geq
            | ast::BinaryOperator::Grt
            | ast::BinaryOperator::Leq
            | ast::BinaryOperator::Less => {
                let left_is_num = left_type == TypeStore::INTEGER || left_type == TypeStore::FLOAT;
                let right_is_num =
                    right_type == TypeStore::INTEGER || right_type == TypeStore::FLOAT;
                if left_type != TypeStore::UNKNOWN && !left_is_num {
                    // Can unwrap safely because `None` results in `UNKNOWN`
                    self.push_binary_error(
                        node.operator,
                        left_type,
                        node.left.as_ref().unwrap().loc(),
                    );
                };
                if right_type != TypeStore::UNKNOWN && !right_is_num {
                    // Can unwrap safely because `None` results in `UNKNOWN`
                    self.push_binary_error(
                        node.operator,
                        right_type,
                        node.right.as_ref().unwrap().loc(),
                    );
                };
                if left_is_num && right_is_num && left_type != right_type {
                    let error = DiagnosticKind::MismatchedTypes {
                        left_name: self.session.display_type(left_type),
                        right_name: self.session.display_type(right_type),
                    };
                    self.error(error, node.loc);
                };
            }
            ast::BinaryOperator::EqEq | ast::BinaryOperator::Neq => {
                let allow_comparison = left_type == right_type
                    || left_type == TypeStore::UNKNOWN
                    || right_type == TypeStore::UNKNOWN;
                if !allow_comparison {
                    let error = DiagnosticKind::MismatchedTypes {
                        left_name: self.session.display_type(left_type),
                        right_name: self.session.display_type(right_type),
                    };
                    self.error(error, node.loc);
                }
            }
            ast::BinaryOperator::LAnd | ast::BinaryOperator::LOr => {
                if left_type != TypeStore::UNKNOWN && left_type != TypeStore::BOOLEAN {
                    // Can unwrap safely because `None` results in `UNKNOWN`
                    self.push_binary_error(
                        node.operator,
                        left_type,
                        node.left.as_ref().unwrap().loc(),
                    );
                };
                if right_type != TypeStore::UNKNOWN && right_type != TypeStore::BOOLEAN {
                    // Can unwrap safely because `None` results in `UNKNOWN`
                    self.push_binary_error(
                        node.operator,
                        right_type,
                        node.right.as_ref().unwrap().loc(),
                    );
                };
            }
        };

        self.ctx.save_expression_type(
            node.loc,
            get_binary_expression_type(node.operator, left_type, right_type),
        )
    }

    fn push_binary_error(&mut self, op: ast::BinaryOperator, ty: TypeId, loc: Location) {
        let error = DiagnosticKind::InvalidTypeForOperator {
            operator: op,
            type_name: self.session.display_type(ty),
        };
        self.error(error, loc)
    }
}

fn get_binary_expression_type(op: ast::BinaryOperator, left: TypeId, right: TypeId) -> TypeId {
    match op {
        ast::BinaryOperator::Add => match (left, right) {
            (TypeStore::STRING, TypeStore::STRING) => TypeStore::STRING,
            (TypeStore::INTEGER, TypeStore::INTEGER) => TypeStore::INTEGER,
            (TypeStore::FLOAT, TypeStore::FLOAT) => TypeStore::FLOAT,
            _ => TypeStore::UNKNOWN,
        },
        ast::BinaryOperator::Sub
        | ast::BinaryOperator::Mul
        | ast::BinaryOperator::Div
        | ast::BinaryOperator::Mod
        | ast::BinaryOperator::Pow => match (left, right) {
            (TypeStore::INTEGER, TypeStore::INTEGER) => TypeStore::INTEGER,
            (TypeStore::FLOAT, TypeStore::FLOAT) => TypeStore::FLOAT,
            _ => TypeStore::UNKNOWN,
        },
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

#[cfg(test)]
mod tests {
    use crate::{type_checker::test_utils::MockLoader, Diagnostic, Session};

    use super::*;

    fn visit_binary_expression(node: ast::BinaryExpression) -> (TypeId, Vec<Diagnostic>) {
        let session = Session::new(Box::new(MockLoader));
        let mut checker = TypeChecker::new(&session, 0);
        let ty = checker.visit_binary_expression(&node);
        (ty, checker.diagnostics)
    }

    #[test]
    fn test_arithmetic_expression() {
        let (ty, errors) = visit_binary_expression(ast::BinaryExpression {
            loc: Location::dummy(),
            operator: ast::BinaryOperator::Add,
            left: Some(Box::new(ast::Expression::IntLiteral(ast::IntLiteral {
                value: 1,
                loc: Location::dummy(),
            }))),
            right: Some(Box::new(ast::Expression::IntLiteral(ast::IntLiteral {
                value: 2,
                loc: Location::dummy(),
            }))),
        });
        assert_eq!(ty, TypeStore::INTEGER);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_invalid_arithmetic_expression() {
        let (ty, errors) = visit_binary_expression(ast::BinaryExpression {
            loc: Location::dummy(),
            operator: ast::BinaryOperator::Add,
            left: Some(Box::new(ast::Expression::IntLiteral(ast::IntLiteral {
                value: 1,
                loc: Location::dummy(),
            }))),
            right: Some(Box::new(ast::Expression::FloatLiteral(ast::FloatLiteral {
                value: ordered_float::OrderedFloat(2.0),
                loc: Location::dummy(),
            }))),
        });
        assert_eq!(ty, TypeStore::UNKNOWN);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            DiagnosticKind::MismatchedTypes { .. }
        ));
    }

    #[test]
    fn test_string_concat() {
        let (ty, errors) = visit_binary_expression(ast::BinaryExpression {
            loc: Location::dummy(),
            operator: ast::BinaryOperator::Add,
            left: Some(Box::new(ast::Expression::StringLiteral(
                ast::StringLiteral {
                    text: "hello".to_string(),
                    loc: Location::dummy(),
                },
            ))),
            right: Some(Box::new(ast::Expression::StringLiteral(
                ast::StringLiteral {
                    text: "world".to_string(),
                    loc: Location::dummy(),
                },
            ))),
        });
        assert_eq!(ty, TypeStore::STRING);
        assert_eq!(errors.len(), 0);
    }
}
