use std::collections::HashMap;

use anyhow::{anyhow, Result};

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
        if let Some(callee) = &node.callee {
            if let ast::Expression::Identifier(id) = callee.as_ref() {
                match id.as_str() {
                    "derived$" => return self.visit_derived_call(node),
                    _ => {}
                }
            }
        }

        let Ok((callee_type, type_params)) = self.resolve_callee(&node.callee) else {
            return self.ctx.save_expression_type(node.loc, TypeStore::UNKNOWN);
        };

        let mut substitutions =
            self.get_explicit_substitutions(&node.type_args, &type_params, node.loc);

        if node.args.len() != callee_type.params.len() {
            let error = DiagnosticKind::ArgumentCountMismatch {
                expected: callee_type.params.len(),
                got: node.args.len(),
            };
            self.error(error, node.loc);
        }

        for (param, arg) in callee_type.params.iter().zip(node.args.iter()) {
            self.check_argument(arg, *param, &mut substitutions);
        }

        let resolved_type_args = callee_type
            .params
            .iter()
            .map(|p| match self.resolve(*p) {
                // TODO: Report error if cannot infer type completely
                types::Type::Param(p) => substitutions.get(&p).unwrap_or(&TypeStore::UNKNOWN),
                _ => p,
            })
            .cloned()
            .collect::<Vec<_>>();

        let return_type = self
            .session
            .types()
            .substitute(callee_type.return_type, &resolved_type_args);

        self.ctx.save_expression_type(node.loc, return_type)
    }

    /// Tries to resolve the type of the function being called.
    /// If something goes wrong (eg the callee is not a function), returns an error.
    /// Errors are reported here if needed (eg "not callable" if wrong type, nothing if `unknown`, etc.)
    /// If the callee is a function, returns its type and any expected type params.
    fn resolve_callee(
        &mut self,
        callee: &Option<Box<ast::Expression>>,
    ) -> Result<(types::FunctionType, Vec<TypeId>)> {
        let callee_type_id = callee
            .as_ref()
            .map(|c| self.visit_expression(c))
            .unwrap_or(TypeStore::UNKNOWN);
        match self.resolve(callee_type_id) {
            types::Type::Function(t) => return Ok((t.clone(), vec![])),
            types::Type::Generic(g) => match self.resolve(g.definition) {
                types::Type::Function(f) => return Ok((f.clone(), g.params)),
                types::Type::Unknown => return Err(anyhow!("")),
                _ => {}
            },
            types::Type::Unknown => return Err(anyhow!("")),
            _ => {}
        };
        let error = DiagnosticKind::NotCallable {
            type_name: self.session.display_type(callee_type_id),
        };
        // unwrapping is safe because `None` callee results in an `unknown` type
        // which is handled above
        self.error(error, callee.as_ref().unwrap().loc());
        Err(anyhow!(""))
    }

    fn check_argument(
        &mut self,
        node: &ast::CallArgument,
        expected: TypeId,
        substitutions: &mut HashMap<types::TypeParam, TypeId>,
    ) {
        match node {
            ast::CallArgument::Expression(expr) => {
                self.check_expression_against(expr, expected, substitutions)
            }
            ast::CallArgument::Callback(node) => self.check_callback(node, expected, substitutions),
        }
    }

    fn check_callback(
        &mut self,
        node: &ast::Callback,
        expected_id: TypeId,
        substitutions: &mut HashMap<types::TypeParam, TypeId>,
    ) {
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
            if let Some(body) = &node.body {
                match &**body {
                    ast::Expression::Block(b) => {
                        s.visit_callback_body(&b, return_type, substitutions)
                    }
                    _ => {
                        let actual_return_type = s.visit_expression(body);
                        s.unify(return_type, actual_return_type, body.loc(), substitutions);
                    }
                }
            };
        });
    }

    pub fn visit_callback_body(
        &mut self,
        body: &ast::BlockExpression,
        expected_type: TypeId,
        substitutions: &mut HashMap<types::TypeParam, TypeId>,
    ) {
        let body_type = self.visit_block_expression(&body);
        self.unify(expected_type, body_type, body.loc, substitutions);
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

    fn visit_derived_call(&mut self, node: &ast::CallExpression) -> TypeId {
        if self.lookup("derived$").is_none() {
            let error = DiagnosticKind::CannotFindName {
                name: "derived$".to_string(),
            };
            // unwrapping is safe because callee has to be `Some` identifier for this function to be called.
            self.error(error, node.callee.as_ref().unwrap().loc());
        }

        if node.args.len() != 1 {
            let error = DiagnosticKind::ArgumentCountMismatch {
                expected: 1,
                got: node.args.len(),
            };
            self.error(error, node.loc);
        }

        let (ty, deps) = match node.args.first() {
            Some(ast::CallArgument::Expression(e)) => {
                self.with_dependencies(|s| s.visit_expression(e))
            }
            Some(ast::CallArgument::Callback(c)) => {
                let error = DiagnosticKind::UnexpectedCallback {
                    expected: "expression".to_string(),
                };
                self.error(error, c.loc);
                (TypeStore::UNKNOWN, vec![])
            }
            None => (TypeStore::UNKNOWN, vec![]),
        };

        let count = self.save_reactive_dependencies(&deps, node.loc);
        if count == 0 {
            self.error(DiagnosticKind::NonReactiveExpression, node.loc);
        }
        self.ctx.add_dependencies(deps);

        let return_type = self.intern(types::Type::Listener(types::ListenerType { inner: ty }));
        self.ctx.save_expression_type(node.loc, return_type)
    }
}
