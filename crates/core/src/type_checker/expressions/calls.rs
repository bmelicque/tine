use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};

use crate::{
    ast,
    diagnostics::DiagnosticKind,
    ir,
    type_checker::{
        analysis_context::{type_store::TypeStore, SymbolData},
        TypeChecker,
    },
    types::{self, Type, TypeId},
    Location, SymbolKind,
};

impl TypeChecker<'_> {
    pub fn visit_call_expression(
        &mut self,
        node: ast::CallExpression,
    ) -> Option<ir::CallExpression> {
        if let Some(callee) = &node.callee {
            if let ast::Expression::Identifier(id) = callee.as_ref() {
                match id.as_str() {
                    "derived$" => return self.visit_derived_call(node),
                    _ => {}
                }
            }
        }

        let Ok((callee, callee_type, type_params)) = self.resolve_callee(node.callee) else {
            return None;
        };

        let (_, mut substitutions) = self.visit_type_args(node.type_args, &type_params, node.loc);

        let args =
            self.check_arguments(node.args, &callee_type.params, &mut substitutions, node.loc);

        let ty = self.resolve_return_type(&type_params, callee_type.return_type, &substitutions);

        Some(ir::CallExpression {
            loc: node.loc,
            callee: Box::new(callee),
            args,
            ty,
        })
    }

    /// Tries to resolve the type of the function being called.
    /// If something goes wrong (eg the callee is not a function), returns an error.
    /// Errors are reported here if needed (eg "not callable" if wrong type, nothing if `unknown`, etc.)
    /// If the callee is a function, returns its type and any expected type params.
    fn resolve_callee(
        &mut self,
        callee: Option<Box<ast::Expression>>,
    ) -> Result<(ir::Expression, types::FunctionType, Vec<TypeId>)> {
        let Some(callee) = callee.and_then(|c| self.visit_expression(*c)) else {
            bail!("");
        };

        match self.resolve(callee.ty()) {
            types::Type::Function(t) => return Ok((callee, t.clone(), vec![])),
            types::Type::Generic(g) => match self.resolve(g.definition) {
                types::Type::Function(f) => return Ok((callee, f.clone(), g.params)),
                types::Type::Unknown => return Err(anyhow!("")),
                _ => {}
            },
            types::Type::Unknown => return Err(anyhow!("")),
            _ => {}
        };
        let error = DiagnosticKind::NotCallable {
            type_name: self.session.display_type(callee.ty()),
        };
        self.error(error, callee.loc());
        Err(anyhow!(""))
    }

    fn check_arguments(
        &mut self,
        args: Vec<ast::CallArgument>,
        params: &[TypeId],
        substitutions: &mut HashMap<types::TypeParam, TypeId>,
        node_loc: Location,
    ) -> Vec<ir::Expression> {
        if args.len() != params.len() {
            let error = DiagnosticKind::ArgumentCountMismatch {
                expected: params.len(),
                got: args.len(),
            };
            self.error(error, node_loc);
        }

        params
            .iter()
            .zip(args.into_iter())
            .filter_map(|(param, arg)| self.check_argument(arg, *param, substitutions))
            .collect()
    }

    fn check_argument(
        &mut self,
        node: ast::CallArgument,
        expected: TypeId,
        substitutions: &mut HashMap<types::TypeParam, TypeId>,
    ) -> Option<ir::Expression> {
        match node {
            ast::CallArgument::Expression(expr) => self
                .check_expression_against(expr, expected, substitutions)
                .map(|e| e.into()),
            ast::CallArgument::Callback(node) => self
                .check_callback(node, expected, substitutions)
                .map(|c| c.into()),
        }
    }

    fn check_callback(
        &mut self,
        node: ast::Callback,
        expected_id: TypeId,
        substitutions: &mut HashMap<types::TypeParam, TypeId>,
    ) -> Option<ir::FunctionExpression> {
        let expected = self.resolve(expected_id);
        let Type::Function(expected) = expected else {
            let error = DiagnosticKind::UnexpectedCallback {
                expected: self.session.display_type(expected_id),
            };
            self.error(error, node.loc);
            return None;
        };

        let (params, body) = self.with_scope(|s| {
            let params = expected.params.clone();
            let return_type = expected.return_type;
            if params.len() != node.params.len() {
                let error = DiagnosticKind::CallbackParamCountMismatch {
                    expected: params.len(),
                    got: node.params.len(),
                };
                s.error(error, node.loc);
            }
            let params = s.visit_callback_params(node.params, &params);
            let Some(body) = node.body else {
                return (params, None);
            };
            let body = match *body {
                ast::Expression::Block(b) => s.visit_callback_body(b, return_type, substitutions),
                body => {
                    let body = s.visit_expression(body);
                    if let Some(body) = &body {
                        s.unify(return_type, body.ty(), body.loc(), substitutions);
                    }
                    body.map(Into::into)
                }
            };
            (params, body)
        });

        let body = body?;

        Some(ir::FunctionExpression {
            loc: node.loc,
            name: None,
            params: params?,
            body,
            ty: expected_id,
        })
    }

    pub fn visit_callback_body(
        &mut self,
        body: ast::BlockExpression,
        expected_type: TypeId,
        substitutions: &mut HashMap<types::TypeParam, TypeId>,
    ) -> Option<ir::Block> {
        let body_type = self.visit_block_expression(body);
        self.unify(expected_type, body_type.ty, body_type.loc, substitutions);
        let returns = body_type.find_returns();
        for ret in returns {
            let ty = ret.expression.map_or(TypeStore::UNIT, |r| r.ty());
            self.check_assigned_type(expected_type, ty, ret.loc);
        }

        self.check_assigned_type(expected_type, body_type.ty, body_type.loc);

        Some(body_type)
    }

    fn visit_callback_params(
        &mut self,
        got: Vec<ast::CallbackParam>,
        expected: &Vec<TypeId>,
    ) -> Option<Vec<ir::Identifier>> {
        got.into_iter()
            .zip(expected.iter())
            .map(|(got, expected)| self.visit_callback_param(got, *expected))
            .collect()
    }

    fn visit_callback_param(
        &mut self,
        got: ast::CallbackParam,
        expected: TypeId,
    ) -> Option<ir::Identifier> {
        match got {
            ast::CallbackParam::Identifier(id) => {
                let symbol = self.ctx.register_symbol(SymbolData {
                    name: id.as_str().into(),
                    ty: expected,
                    kind: SymbolKind::constant(),
                    defined_at: id.loc,
                    ..Default::default()
                });
                Some(ir::Identifier {
                    loc: id.loc,
                    symbol,
                })
            }
            ast::CallbackParam::Param(param) => {
                let type_annotation = self.visit_type(param.type_annotation.unwrap());
                let name = param.name.as_str().into();
                let kind = SymbolKind::constant();
                let defined_at = param.name.loc;
                match type_annotation {
                    TypeStore::UNKNOWN => {
                        let ty = expected;
                        let symbol = self.ctx.register_symbol(SymbolData {
                            name,
                            ty,
                            kind,
                            defined_at,
                            ..Default::default()
                        });
                        Some(ir::Identifier {
                            loc: defined_at,
                            symbol,
                        })
                    }
                    ty => {
                        let symbol = self.ctx.register_symbol(SymbolData {
                            name,
                            ty,
                            kind,
                            defined_at,
                            ..Default::default()
                        });
                        if ty != expected {
                            let error = DiagnosticKind::MismatchedTypes {
                                left_name: self.session.display_type(expected),
                                right_name: self.session.display_type(ty),
                            };
                            self.error(error, defined_at);
                        }
                        Some(ir::Identifier {
                            loc: param.name.loc,
                            symbol,
                        })
                    }
                }
            }
        }
    }

    fn resolve_return_type(
        &mut self,
        type_params: &[TypeId],
        return_type: TypeId,
        substitutions: &HashMap<types::TypeParam, TypeId>,
    ) -> TypeId {
        let resolved_type_args = type_params
            .iter()
            .map(|p| match self.resolve(*p) {
                // TODO: Report error if cannot infer type completely
                types::Type::Param(p) => substitutions.get(&p).unwrap_or(&TypeStore::UNKNOWN),
                _ => p,
            })
            .cloned()
            .collect::<Vec<_>>();

        self.session
            .types()
            .substitute(return_type, &resolved_type_args)
    }

    fn visit_derived_call(&mut self, node: ast::CallExpression) -> Option<ir::CallExpression> {
        let callee = node.callee.and_then(|e| self.visit_expression(*e))?;

        if node.args.len() != 1 {
            let error = DiagnosticKind::ArgumentCountMismatch {
                expected: 1,
                got: node.args.len(),
            };
            self.error(error, node.loc);
            return None;
        }

        let (arg, mut deps) = match node.args.into_iter().next() {
            Some(ast::CallArgument::Expression(e)) => {
                let (arg, deps) = self.with_dependencies(|s| s.visit_expression(e));
                (arg?, deps)
            }
            Some(ast::CallArgument::Callback(c)) => {
                let error = DiagnosticKind::UnexpectedCallback {
                    expected: "expression".to_string(),
                };
                self.error(error, c.loc);
                return None;
            }
            // caught by length check above
            None => unreachable!(),
        };
        self.ctx.add_dependencies(deps.clone());
        deps.retain(|dep| self.resolve(dep.borrow().get_type()).is_reactive());
        if deps.len() == 0 {
            self.error(DiagnosticKind::NonReactiveExpression, node.loc);
        }
        let deps: Vec<ir::Expression> = deps
            .into_iter()
            .map(|symbol| {
                ir::Expression::Identifier(ir::Identifier {
                    loc: node.loc,
                    symbol,
                })
            })
            .collect();
        let dependency_array = ir::Expression::Tuple(ir::TupleExpression {
            loc: node.loc,
            ty: self.intern(types::TupleType {
                elements: deps.iter().map(|e| e.ty()).collect(),
            }),
            elements: deps,
        });

        let return_type = self.intern(types::Type::Listener(types::ListenerType {
            inner: arg.ty(),
        }));

        Some(ir::CallExpression {
            loc: node.loc,
            callee: Box::new(callee),
            args: vec![arg.into(), dependency_array.into()],
            ty: return_type,
        })
    }
}
