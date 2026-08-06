use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};
use tine_ir::{self as ir, Typed};
use tine_types::{
    store::TypeStore,
    types::{self, TypeId},
};

use crate::TypeChecker;

const VALID_ADD_TYPES: [TypeId; 3] = [TypeStore::INTEGER, TypeStore::FLOAT, TypeStore::STRING];

impl TypeChecker {
    pub fn visit_binary_expression(
        &mut self,
        node: ast::BinaryExpression,
    ) -> Option<ir::BinaryExpression> {
        let left = node.left.and_then(|e| self.visit_expression(*e));
        let right = node.right.and_then(|e| self.visit_expression(*e));
        let (Some(left), Some(right)) = (left, right) else {
            return None;
        };
        let left_type = left.ty();
        let right_type = right.ty();

        self.validate_binary_operands(node.operator, &left, &right, node.loc);

        let ty = get_binary_expression_type(node.operator, left_type, right_type);

        Some(ir::BinaryExpression {
            loc: node.loc,
            left: Box::new(left),
            right: Box::new(right),
            op: node.operator,
            ty,
        })
    }

    fn validate_binary_operands(
        &mut self,
        operator: ast::BinaryOperator,
        left: &ir::Expression,
        right: &ir::Expression,
        node_loc: Location,
    ) {
        use ast::BinaryOperator::*;
        match operator {
            Add => {
                let left_is_ok = VALID_ADD_TYPES.contains(&left.ty());
                let right_is_ok = VALID_ADD_TYPES.contains(&right.ty());
                if !left_is_ok && left.ty() != TypeStore::UNKNOWN {
                    self.push_binary_error(operator, left.ty(), left.loc());
                };
                if !right_is_ok && right.ty() != TypeStore::UNKNOWN {
                    self.push_binary_error(operator, right.ty(), right.loc());
                };
                if left_is_ok && right_is_ok && left.ty() != right.ty() {
                    let error = DiagnosticKind::MismatchedTypes {
                        left_name: self.types.display(left.ty()),
                        right_name: self.types.display(right.ty()),
                    };
                    self.error(error, node_loc);
                };
            }
            Sub | Mul | Div | Mod | Pow | Geq | Grt | Leq | Less => {
                let left_is_num = left.ty() == TypeStore::INTEGER || left.ty() == TypeStore::FLOAT;
                let right_is_num =
                    right.ty() == TypeStore::INTEGER || right.ty() == TypeStore::FLOAT;
                if left.ty() != TypeStore::UNKNOWN && !left_is_num {
                    self.push_binary_error(operator, left.ty(), left.loc());
                };
                if right.ty() != TypeStore::UNKNOWN && !right_is_num {
                    self.push_binary_error(operator, right.ty(), right.loc());
                };
                if left_is_num && right_is_num && left.ty() != right.ty() {
                    let error = DiagnosticKind::MismatchedTypes {
                        left_name: self.types.display(left.ty()),
                        right_name: self.types.display(right.ty()),
                    };
                    self.error(error, node_loc);
                };
            }
            EqEq | Neq => self.validate_eq_operands(left, right, node_loc),
            LAnd | LOr => {
                if left.ty() != TypeStore::UNKNOWN && left.ty() != TypeStore::BOOLEAN {
                    self.push_binary_error(operator, left.ty(), left.loc());
                };
                if right.ty() != TypeStore::UNKNOWN && right.ty() != TypeStore::BOOLEAN {
                    self.push_binary_error(operator, right.ty(), right.loc());
                };
            }
        };
    }

    fn validate_eq_operands(
        &mut self,
        left: &ir::Expression,
        right: &ir::Expression,
        node_loc: Location,
    ) {
        if left.ty() == TypeStore::UNKNOWN || right.ty() == TypeStore::UNKNOWN {
            return;
        }
        let allow_comparison = left.ty() == right.ty() && self.is_equatable(left.ty());
        if !allow_comparison {
            let error = DiagnosticKind::MismatchedTypes {
                left_name: self.types.display(left.ty()),
                right_name: self.types.display(right.ty()),
            };
            self.error(error, node_loc);
        }
    }

    /// Return `true` if given type can be used with the `==` operator
    fn is_equatable(&self, ty: types::TypeId) -> bool {
        use types::Type::*;
        match self.resolve(ty) {
            Boolean | Integer | Float | String => true,
            Ref(r) if r.inner == TypeStore::ARRAY => self.is_equatable(r.args[0]),
            Tuple(t) => t.elements.into_iter().all(|e| self.is_equatable(e)),
            _ => false,
        }
    }

    fn push_binary_error(&mut self, op: ast::BinaryOperator, ty: TypeId, loc: Location) {
        let error = DiagnosticKind::InvalidTypeForOperator {
            operator: op.to_string(),
            type_name: self.types.display(ty),
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
    use tine_common::diagnostics::Diagnostic;

    use super::*;

    fn visit_binary_expression(node: ast::BinaryExpression) -> (TypeId, Vec<Diagnostic>) {
        let mut checker = TypeChecker::new();
        let ty = checker
            .visit_binary_expression(node)
            .map_or(TypeStore::UNKNOWN, |n| n.ty);
        let diagnostics = checker.diagnostics.get(&0).map_or(vec![], |d| d.clone());
        (ty, diagnostics)
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
