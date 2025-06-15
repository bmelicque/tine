use crate::{ast, parser::parser::ParseError, type_checker::TypeChecker, types};

impl TypeChecker {
    pub fn visit_call_expression(&mut self, node: &ast::CallExpression) -> types::Type {
        let callee_type = self.visit_expression(&node.callee);
        let types::Type::Function(callee_type) = callee_type else {
            self.errors.push(ParseError {
                message: format!("Type '{}' is not callable", callee_type),
                span: node.span,
            });
            return self.set_type_at(node.span, types::Type::Unknown);
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
        if !got.is_assignable_to(&expected) {
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

        self.symbols.enter_scope();

        if expected.params.len() != node.params.len() {
            self.errors.push(ParseError {
                message: format!(
                    "expected {} param(s), got {}",
                    expected.params.len(),
                    node.params.len()
                ),
                span: node.span,
            });
        }
        self.define_params(&node.params, &expected.params);
        let body_type = self.visit_function_body(&node.body);
        if !body_type.is_assignable_to(&expected.return_type) {
            self.errors.push(ParseError {
                message: format!("Expected type {}, got {}", expected.return_type, body_type),
                span: node.span,
            })
        }

        self.symbols.exit_scope();
    }

    fn define_params(&mut self, got: &Vec<ast::PredicateParam>, expected: &Vec<types::Type>) {
        for (i, param) in got.iter().take(expected.len()).enumerate() {
            match param {
                ast::PredicateParam::Identifier(id) => {
                    let ty = expected[i].clone();
                    self.symbols.define(id.as_str(), ty.clone(), false);
                }
                ast::PredicateParam::Param(param) => {
                    let ty = self.resolve_type(&param.type_annotation);
                    self.symbols.define(param.name.as_str(), ty.clone(), false);
                }
            }
        }
    }
}
