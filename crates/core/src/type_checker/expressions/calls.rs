use crate::{
    ast,
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
            t => {
                self.error(format!("type '{}' is not callable", t), node.callee.loc());
                return self.ctx.save_expression_type(node.loc, TypeStore::UNKNOWN);
            }
        };

        if node.args.len() != callee_type.params.len() {
            let error_message = format!(
                "expected {} argument(s), got {}",
                callee_type.params.len(),
                node.args.len()
            );
            self.error(error_message, node.loc);
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

    fn check_callback(&mut self, node: &ast::Callback, expected: TypeId) {
        let expected = self.resolve(expected);
        let Type::Function(expected) = expected else {
            self.error(
                format!("Expected type {}, got function", expected),
                node.loc,
            );
            return;
        };
        let params = expected.params.clone();
        let return_type = expected.return_type;

        self.with_scope(|s| {
            if params.len() != node.params.len() {
                let message = format!(
                    "expected {} param(s), got {}",
                    params.len(),
                    node.params.len()
                );
                s.error(message, node.loc);
            }
            s.define_params(&node.params, &params);
            s.visit_callback_body(node, return_type);
        });
    }

    pub fn visit_callback_body(&mut self, node: &ast::Callback, expected_type: TypeId) {
        let body_type = self.visit_block_expression(&node.body);
        let mut returns = Vec::<ast::ReturnStatement>::new();
        node.body.find_returns(&mut returns);
        for ret in returns {
            let ty = match ret.value {
                Some(value) => self.get_type_at(value.loc()).unwrap(),
                None => TypeStore::UNIT,
            };
            self.check_assigned_type(expected_type, ty, ret.loc);
        }

        if let Some(ast::Statement::Expression(expr)) = node.body.statements.last() {
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
                    let ty = self.visit_type(&param.type_annotation);
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
