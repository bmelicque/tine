use anyhow::{anyhow, bail, Result};
use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};
use tine_ir::{self as ir, Typed};
use tine_symbols::symbols::*;
use tine_types::{store::TypeStore, types};

use crate::{substitutions::Substitutions, TypeChecker};

impl TypeChecker {
    pub fn visit_call_expression(
        &mut self,
        node: ast::CallExpression,
    ) -> Option<ir::CallExpression> {
        if let Some(callee) = &node.callee {
            if let ast::Expression::Identifier(id) = callee.as_ref() {
                match id.as_str() {
                    "computed$" => return self.visit_derived_call(node),
                    _ => {}
                }
            }
        }

        let Ok((callee, callee_type)) = self.resolve_callee(node.callee) else {
            return None;
        };

        let (_, mut substitutions) =
            self.visit_type_args(node.type_args, &callee_type.type_params, node.loc);

        let args =
            self.check_arguments(node.args, &callee_type.params, &mut substitutions, node.loc);

        let ty = substitutions.apply(&mut self.types, callee_type.return_type);

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
    fn resolve_callee(
        &mut self,
        callee: Option<Box<ast::Expression>>,
    ) -> Result<(ir::Expression, types::FunctionType)> {
        let Some(callee) = callee.and_then(|c| self.visit_expression(*c)) else {
            bail!("");
        };

        match self.resolve(callee.ty()) {
            types::Type::Function(t) => Ok((callee, t)),
            types::Type::Unknown => Err(anyhow!("")),
            _ => {
                let error = DiagnosticKind::NotCallable {
                    type_name: self.types.display(callee.ty()),
                };
                self.error(error, callee.loc());
                Err(anyhow!(""))
            }
        }
    }

    fn check_arguments(
        &mut self,
        args: Vec<ast::CallArgument>,
        params: &[types::TypeId],
        substitutions: &mut Substitutions,
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
        expected: types::TypeId,
        substitutions: &mut Substitutions,
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
        expected_id: types::TypeId,
        substitutions: &mut Substitutions,
    ) -> Option<ir::FunctionExpression> {
        let expected = self.resolve(expected_id);
        let types::Type::Function(expected) = expected else {
            let error = DiagnosticKind::UnexpectedCallback {
                expected: self.types.display(expected_id),
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
                        substitutions.unify(s, return_type, body.ty(), body.loc());
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
        expected_type: types::TypeId,
        substitutions: &mut Substitutions,
    ) -> Option<ir::Block> {
        let body_type = self.visit_block_expression(body);
        substitutions.unify(self, expected_type, body_type.ty, body_type.loc);
        let returns = body_type.find_returns();
        for ret in returns {
            let ty = ret.expression.as_ref().map_or(TypeStore::UNIT, |r| r.ty());
            self.check_assigned_type(expected_type, ty, true, ret.loc);
        }

        self.check_assigned_type(expected_type, body_type.ty, true, body_type.loc);

        Some(body_type)
    }

    fn visit_callback_params(
        &mut self,
        got: Vec<ast::CallbackParam>,
        expected: &Vec<types::TypeId>,
    ) -> Option<Vec<(Location, VariableSymbolId)>> {
        got.into_iter()
            .zip(expected.iter())
            .map(|(got, expected)| self.visit_callback_param(got, *expected))
            .collect()
    }

    fn visit_callback_param(
        &mut self,
        got: ast::CallbackParam,
        expected: types::TypeId,
    ) -> Option<(Location, VariableSymbolId)> {
        match got {
            ast::CallbackParam::Identifier(id) => {
                let symbol = self.symbols.insert(VariableSymbol {
                    name: id.text,
                    ty: expected,
                    defined_at: id.loc,
                    ..Default::default()
                });
                Some((id.loc, symbol))
            }
            ast::CallbackParam::Param(param) => {
                let id = param.name?;
                let type_annotation = self.visit_type(param.type_annotation.unwrap());
                let name = id.as_str().into();
                let defined_at = id.loc;
                match type_annotation {
                    TypeStore::UNKNOWN => {
                        let ty = expected;
                        let symbol = self.symbols.insert(VariableSymbol {
                            name,
                            ty,
                            defined_at,
                            ..Default::default()
                        });
                        Some((defined_at, symbol))
                    }
                    ty => {
                        let symbol = self.symbols.insert(VariableSymbol {
                            name,
                            ty,
                            defined_at,
                            ..Default::default()
                        });
                        if ty != expected {
                            let error = DiagnosticKind::MismatchedTypes {
                                left_name: self.types.display(expected),
                                right_name: self.types.display(ty),
                            };
                            self.error(error, defined_at);
                        }
                        Some((id.loc, symbol))
                    }
                }
            }
        }
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

        let arg = match node.args.into_iter().next() {
            Some(ast::CallArgument::Expression(e)) => self.visit_expression(e)?,
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
        let deps = self
            .dependencies(&arg)
            .filter(|dep| self.symbol_type(dep.symbol).is_reactive())
            .cloned()
            .collect::<Vec<_>>();
        if deps.len() == 0 {
            self.error(DiagnosticKind::NonReactiveExpression, node.loc);
        }
        let dependency_array = ir::Expression::Tuple(ir::TupleExpression {
            loc: node.loc,
            ty: self.intern(types::TupleType {
                elements: deps.iter().map(|e| e.ty).collect(),
                ..Default::default()
            }),
            elements: deps.into_iter().map(Into::into).collect(),
        });

        let return_type = self.intern(types::Type::Listener(types::ListenerType {
            inner: arg.ty(),
        }));

        let arg = ir::FunctionExpression {
            ty: self.intern(types::FunctionType {
                return_type: arg.ty(),
                ..Default::default()
            }),
            loc: arg.loc(),
            name: None,
            params: vec![],
            body: ir::Block {
                ty: arg.ty(),
                loc: arg.loc(),
                statements: vec![ir::Statement::Return(ir::ReturnStatement {
                    loc: arg.loc(),
                    expression: Some(Box::new(arg)),
                })],
            },
        };

        Some(ir::CallExpression {
            loc: node.loc,
            callee: Box::new(callee),
            args: vec![arg.into(), dependency_array.into()],
            ty: return_type,
        })
    }
}
