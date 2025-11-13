use crate::{
    ast,
    parser::parser::ParseError,
    type_checker::{analysis_context::VariableData, TypeChecker},
    types,
};

impl TypeChecker {
    pub fn visit_call_expression(&mut self, node: &ast::CallExpression) -> types::Type {
        let callee_type = match self.visit_expression(&node.callee) {
            types::Type::Function(t) => t,
            types::Type::Unknown => {
                return self.set_type_at(node.span, types::Type::Unknown);
            }
            t => {
                self.error(
                    format!("type '{}' is not callable", t),
                    node.callee.as_span(),
                );
                return self.set_type_at(node.span, types::Type::Unknown);
            }
        };

        if node.args.len() != callee_type.params.len() {
            self.errors.push(ParseError {
                message: format!(
                    "expected {} argument(s), got {}",
                    callee_type.params.len(),
                    node.args.len()
                ),
                span: node.span,
            });
        }

        node.args
            .iter()
            .enumerate()
            .take(callee_type.params.len())
            .for_each(|(i, param)| {
                self.check_argument(param, callee_type.params[i].clone());
            });

        self.set_type_at(node.span, *callee_type.return_type)
    }

    fn check_argument(&mut self, node: &ast::CallArgument, expected: types::Type) {
        match node {
            ast::CallArgument::Expression(expr) => self.check_expression_argument(expr, expected),
            ast::CallArgument::Predicate(node) => self.check_predicate(node, expected),
        }
    }

    fn check_expression_argument(&mut self, node: &ast::Expression, expected: types::Type) {
        let got = self.visit_expression(node);
        if !self.can_be_assigned_to(&got, &expected) {
            self.errors.push(ParseError {
                message: format!("Expected type {}, got {}", expected, got),
                span: node.as_span(),
            })
        }
    }

    fn check_predicate(&mut self, node: &ast::Predicate, expected: types::Type) {
        let types::Type::Function(expected) = expected else {
            self.errors.push(ParseError {
                message: format!("Expected type {}, got function", expected),
                span: node.span,
            });
            return;
        };

        self.with_scope(node.span, |s| {
            if expected.params.len() != node.params.len() {
                s.errors.push(ParseError {
                    message: format!(
                        "expected {} param(s), got {}",
                        expected.params.len(),
                        node.params.len()
                    ),
                    span: node.span,
                });
            }
            s.define_params(&node.params, &expected.params);
            let body_type = s.visit_function_body(&node.body);
            if !s.can_be_assigned_to(&body_type, &expected.return_type) {
                s.errors.push(ParseError {
                    message: format!("Expected type {}, got {}", expected.return_type, body_type),
                    span: node.span,
                })
            }
        });
    }

    fn define_params(&mut self, got: &Vec<ast::PredicateParam>, expected: &Vec<types::Type>) {
        for (i, param) in got.iter().take(expected.len()).enumerate() {
            match param {
                ast::PredicateParam::Identifier(id) => {
                    self.analysis_context.register_symbol(VariableData::new(
                        id.as_str().into(),
                        expected[i].clone().into(),
                        false,
                        id.span,
                        vec![],
                    ));
                }
                ast::PredicateParam::Param(param) => {
                    let ty = self.resolve_type(&param.type_annotation);
                    self.analysis_context.register_symbol(VariableData::new(
                        param.name.as_str().into(),
                        ty.into(),
                        false,
                        param.name.span,
                        vec![],
                    ));
                }
            }
        }
    }
}
