use crate::{
    ast::{self, utils::root_identifier},
    parser::parser::ParseError,
    type_checker::TypeChecker,
    types,
};

impl TypeChecker {
    pub fn visit_unary_expression(&mut self, node: &ast::UnaryExpression) -> types::Type {
        match node.operator {
            ast::UnaryOperator::Deref => self.visit_dereference(node),
            ast::UnaryOperator::ImmutableRef => self.visit_immutable_ref(node),
            ast::UnaryOperator::MutableRef => self.visit_mutable_ref(node),
            ast::UnaryOperator::Negate => self.visit_negate_expresion(node),
            ast::UnaryOperator::Not => self.visit_logical_not_expresion(node),
        }
    }

    fn visit_dereference(&mut self, node: &ast::UnaryExpression) -> types::Type {
        let expr_type = self.visit_expression(&node.operand);
        match expr_type {
            types::Type::Reference(inner) => self.set_type_at(node.span, *inner.target.clone()),
            types::Type::Unknown => self.set_type_at(node.span, types::Type::Unknown),
            _ => {
                self.errors.push(ParseError {
                    message: format!("Cannot dereference type {}", expr_type),
                    span: node.span,
                });
                self.set_type_at(node.span, types::Type::Unknown)
            }
        }
    }

    fn visit_immutable_ref(&mut self, node: &ast::UnaryExpression) -> types::Type {
        let expr_type = self.visit_expression(&node.operand);
        if let ast::Expression::Identifier(id) = node.operand.as_ref() {
            if let Some(info) = self.analysis_context.lookup_mut(&id.as_str()) {
                info.ro_refs += 1;
                info.reads -= 1; // previous visit_expression added a read
            };
        }

        self.set_type_at(
            node.span,
            types::Type::Reference(types::ReferenceType {
                target: Box::new(expr_type),
                mutable: false,
            }),
        )
    }

    fn visit_mutable_ref(&mut self, node: &ast::UnaryExpression) -> types::Type {
        let expr_type = self.visit_expression(&node.operand);
        if let ast::Expression::Identifier(id) = node.operand.as_ref() {
            if let Some(info) = self.analysis_context.lookup_mut(&id.as_str()) {
                if !info.mutable {
                    self.errors.push(ParseError {
                        message: format!(
                            "Cannot take mutable reference of immutable variable '{}'",
                            id.as_str()
                        ),
                        span: node.span,
                    });
                } else {
                    info.mut_refs += 1;
                    info.reads -= 1; // previous visit_expression added a read
                }
            };
        } else if let Some(id) = root_identifier(&node.operand) {
            if let Some(info) = self.analysis_context.lookup_mut(&id.as_str()) {
                if !info.mutable {
                    self.errors.push(ParseError {
                        message: format!("Cannot assign to immutable variable '{}'", id.as_str()),
                        span: node.span,
                    });
                } else {
                    info.writes += 1;
                    info.reads -= 1; // previous visit_expression added a read
                }
            };
        };

        self.set_type_at(
            node.span,
            types::Type::Reference(types::ReferenceType {
                target: Box::new(expr_type),
                mutable: true,
            }),
        )
    }

    fn visit_negate_expresion(&mut self, node: &ast::UnaryExpression) -> types::Type {
        let expr_type = self.visit_expression(&node.operand);
        if expr_type != types::Type::Number {
            self.errors.push(ParseError {
                message: format!("Cannot negate type {} (number expected)", expr_type),
                span: node.span,
            });
            return self.set_type_at(node.span, types::Type::Unknown);
        }
        self.set_type_at(node.span, types::Type::Number)
    }

    fn visit_logical_not_expresion(&mut self, node: &ast::UnaryExpression) -> types::Type {
        let expr_type = self.visit_expression(&node.operand);
        if expr_type != types::Type::Boolean {
            self.errors.push(ParseError {
                message: format!(
                    "Cannot apply logical not to type {} (boolean expected)",
                    expr_type
                ),
                span: node.span,
            });
            return self.set_type_at(node.span, types::Type::Unknown);
        }
        self.set_type_at(node.span, types::Type::Boolean)
    }
}
