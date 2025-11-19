use crate::{
    ast,
    type_checker::{
        analysis_context::{type_store::TypeStore, SymbolData},
        SymbolKind, TypeChecker,
    },
    types::{self, Type, TypeId},
};

impl TypeChecker {
    pub fn visit_call_expression(&mut self, node: &ast::CallExpression) -> TypeId {
        let callee_type = self.visit_expression(&node.callee);
        let callee_type = match self.analysis_context.type_store.get(callee_type) {
            types::Type::Function(t) => t.clone(),
            types::Type::Unknown => {
                return self
                    .analysis_context
                    .save_expression_type(node.span, TypeStore::UNKNOWN);
            }
            t => {
                self.error(
                    format!("type '{}' is not callable", t),
                    node.callee.as_span(),
                );
                return self
                    .analysis_context
                    .save_expression_type(node.span, TypeStore::UNKNOWN);
            }
        };

        if node.args.len() != callee_type.params.len() {
            let error_message = format!(
                "expected {} argument(s), got {}",
                callee_type.params.len(),
                node.args.len()
            );
            self.error(error_message, node.span);
        }

        node.args
            .iter()
            .enumerate()
            .take(callee_type.params.len())
            .for_each(|(i, param)| {
                self.check_argument(param, callee_type.params[i].clone());
            });

        self.analysis_context
            .save_expression_type(node.span, callee_type.return_type)
    }

    fn check_argument(&mut self, node: &ast::CallArgument, expected: TypeId) {
        match node {
            ast::CallArgument::Expression(expr) => self.check_expression_argument(expr, expected),
            ast::CallArgument::Predicate(node) => self.check_predicate(node, expected),
        }
    }

    fn check_expression_argument(&mut self, node: &ast::Expression, expected: TypeId) {
        let got = self.visit_expression(node);
        self.check_assigned_type(expected, got, node.as_span());
    }

    fn check_predicate(&mut self, node: &ast::Predicate, expected: TypeId) {
        let expected = self.resolve(expected);
        let Type::Function(expected) = expected else {
            self.error(
                format!("Expected type {}, got function", expected),
                node.span,
            );
            return;
        };
        let params = expected.params.clone();
        let return_type = expected.return_type;

        self.with_scope(node.span, |s| {
            if params.len() != node.params.len() {
                s.error(
                    format!(
                        "expected {} param(s), got {}",
                        params.len(),
                        node.params.len()
                    ),
                    node.span,
                );
            }
            s.define_params(&node.params, &params);
            let body_type = s.visit_function_body(&node.body);
            s.check_assigned_type(return_type, body_type, node.span);
        });
    }

    fn define_params(&mut self, got: &Vec<ast::PredicateParam>, expected: &Vec<TypeId>) {
        for (i, param) in got.iter().take(expected.len()).enumerate() {
            match param {
                ast::PredicateParam::Identifier(id) => {
                    self.analysis_context.register_symbol(SymbolData::new(
                        id.as_str().into(),
                        SymbolKind::Value,
                        expected[i],
                        false,
                        id.span,
                        vec![],
                    ));
                }
                ast::PredicateParam::Param(param) => {
                    let ty = self.visit_type(&param.type_annotation);
                    self.analysis_context.register_symbol(SymbolData::new(
                        param.name.as_str().into(),
                        SymbolKind::Value,
                        ty,
                        false,
                        param.name.span,
                        vec![],
                    ));
                }
            }
        }
    }
}
