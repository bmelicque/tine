use crate::{
    ast,
    diagnostics::DiagnosticKind,
    type_checker::{
        analysis_context::{type_store::TypeStore, SymbolData},
        TypeChecker,
    },
    types::{self, Type, TypeId},
    SymbolKind,
};

impl TypeChecker<'_> {
    pub fn visit_call_expression(&mut self, node: &ast::CallExpression) -> TypeId {
        let callee_type = self.visit_expression(&node.callee);
        let callee_type = match self.resolve(callee_type) {
            types::Type::Function(t) => t.clone(),
            types::Type::Unknown => {
                return self.ctx.save_expression_type(node.loc, TypeStore::UNKNOWN);
            }
            _ => {
                let error = DiagnosticKind::NotCallable {
                    type_name: self.session.display_type(callee_type),
                };
                self.error(error, node.callee.loc());
                return self.ctx.save_expression_type(node.loc, TypeStore::UNKNOWN);
            }
        };

        if node.args.len() != callee_type.params.len() {
            let error = DiagnosticKind::ArgumentCountMismatch {
                expected: callee_type.params.len(),
                got: node.args.len(),
            };
            self.error(error, node.loc);
        }

        node.args
            .iter()
            .enumerate()
            .take(callee_type.params.len())
            .for_each(|(i, param)| {
                self.check_argument(param, callee_type.params[i].clone());
            });

        self.ctx
            .save_expression_type(node.loc, callee_type.return_type)
    }

    fn check_argument(&mut self, node: &ast::CallArgument, expected: TypeId) {
        match node {
            ast::CallArgument::Expression(expr) => self.check_expression_argument(expr, expected),
            ast::CallArgument::Callback(node) => self.check_callback(node, expected),
        }
    }

    fn check_expression_argument(&mut self, node: &ast::Expression, expected: TypeId) {
        let got = self.visit_expression(node);
        self.check_assigned_type(expected, got, node.loc());
    }

    fn check_callback(&mut self, node: &ast::Callback, expected_id: TypeId) {
        let expected = self.resolve(expected_id);
        let Type::Function(expected) = expected else {
            let error = DiagnosticKind::UnexpectedCallback {
                expected: self.session.display_type(expected_id),
            };
            self.error(error, node.loc);
            return;
        };
        let params = expected.params.clone();
        let return_type = expected.return_type;

        self.with_scope(|s| {
            if params.len() != node.params.len() {
                let error = DiagnosticKind::CallbackParamCountMismatch {
                    expected: params.len(),
                    got: node.params.len(),
                };
                s.error(error, node.loc);
            }
            s.define_params(&node.params, &params);
            match &*node.body {
                ast::Expression::Block(b) => s.visit_callback_body(&b, return_type),
                _ => {
                    s.visit_expression(&node.body);
                }
            }
        });
    }

    pub fn visit_callback_body(&mut self, body: &ast::BlockExpression, expected_type: TypeId) {
        let body_type = self.visit_block_expression(&body);
        let mut returns = Vec::<ast::ReturnStatement>::new();
        body.find_returns(&mut returns);
        for ret in returns {
            let ty = match ret.value {
                Some(value) => self.get_type_at(value.loc()).unwrap(),
                None => TypeStore::UNIT,
            };
            self.check_assigned_type(expected_type, ty, ret.loc);
        }

        if let Some(ast::Statement::Expression(expr)) = body.statements.last() {
            if expected_type != TypeStore::UNIT {
                self.check_assigned_type(expected_type, body_type, expr.expression.loc());
            }
        }
    }

    fn define_params(&mut self, got: &Vec<ast::CallbackParam>, expected: &Vec<TypeId>) {
        for (i, param) in got.iter().take(expected.len()).enumerate() {
            match param {
                ast::CallbackParam::Identifier(id) => {
                    self.ctx.register_symbol(SymbolData {
                        name: id.as_str().into(),
                        ty: expected[i],
                        kind: SymbolKind::constant(),
                        defined_at: id.loc,
                        ..Default::default()
                    });
                }
                ast::CallbackParam::Param(param) => {
                    let ty = self.visit_type(param.type_annotation.as_ref().unwrap());
                    self.ctx.register_symbol(SymbolData {
                        name: param.name.as_str().into(),
                        ty,
                        kind: SymbolKind::constant(),
                        defined_at: param.name.loc,
                        ..Default::default()
                    });
                }
            }
        }
    }
}
