use crate::{ast, parser::parser::ParseError, type_checker::TypeChecker, types};

impl TypeChecker {
    pub fn visit_call_expression(&mut self, node: &ast::CallExpression) -> types::Type {
        let callee_type = self.visit_expression(&node.callee);
        node.args.iter().for_each(|param| {
            self.visit_expression(param);
        });
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
                let got = self.get_type_at(param.as_span()).unwrap();
                let expected = &callee_type.params[i];
                if !got.is_assignable_to(expected) {
                    self.errors.push(ParseError {
                        message: format!("Expected type {}, got {}", expected, got),
                        span: param.as_span(),
                    });
                }
            });

        self.set_type_at(node.span, *callee_type.return_type)
    }
}
