use anyhow::{anyhow, bail, Result};
use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};
use tine_ir::{self as ir, Typed};
use tine_symbols::symbols::*;
use tine_types::types;

use crate::{substitutions::Substitutions, TypeChecker};

impl TypeChecker {
    pub fn visit_call_expression(&mut self, node: ast::CallExpression) -> Option<ir::Expression> {
        match node.callee.as_deref() {
            Some(ast::Expression::Identifier(id)) if id.as_str() == "computed$" => {
                return self.visit_derived_call(node).map(Into::into);
            }
            _ => {}
        }

        let Ok((callee, callee_type)) = self.resolve_callee(node.callee) else {
            return None;
        };

        if let ir::Expression::Identifier(ir::Identifier {
            symbol: SymbolId::Variant(v),
            ty,
            loc,
        }) = callee
        {
            return self.visit_variant_literal(v, ty, node.args, loc, node.loc);
        }

        let (_, mut substitutions) =
            self.visit_type_args(node.type_args, &callee_type.type_params, node.loc);

        let args =
            self.check_arguments(node.args, &callee_type.params, &mut substitutions, node.loc);

        let ty = substitutions.apply(&mut self.types, callee_type.return_type);
        self.errors(substitutions.produce_diagnostics(&self.types), node.loc);

        match callee {
            ir::Expression::Method(mut m) => {
                m.args = args;
                m.ty = ty;
                self.cancel_diag(|d| {
                    d.loc == m.method.0 && d.kind == DiagnosticKind::NonCalledMethod
                });
                Some(m.into())
            }
            callee => Some(ir::Expression::Call(ir::CallExpression {
                loc: node.loc,
                callee: Box::new(callee),
                args,
                ty,
            })),
        }
    }

    fn visit_variant_literal(
        &mut self,
        v: VariantSymbolId,
        constructor: types::TypeId,
        args: Vec<ast::Expression>,
        variant_loc: Location,
        loc: Location,
    ) -> Option<ir::Expression> {
        let (constructor, mut sub) = self.unwrap_type(constructor);
        let symbol = self.symbols.get(v);
        let params = symbol
            .body
            .iter()
            .map(|m| self.symbol_type_id(*m))
            .collect::<Vec<_>>();
        let args = self.check_arguments(args, &params, &mut sub, loc);
        let ty = sub.apply(&mut self.types, constructor);
        self.errors(sub.produce_diagnostics(&self.types), variant_loc);
        Some(ir::Expression::Call(ir::CallExpression {
            ty,
            loc,
            callee: Box::new(ir::Expression::Identifier(ir::Identifier {
                ty,
                loc: variant_loc,
                symbol: v.into(),
            })),
            args,
        }))
    }

    /// Tries to resolve the type of the function being called.
    /// If something goes wrong (eg the callee is not a function), returns an error.
    /// Errors are reported here if needed (eg "not callable" if wrong type, nothing if `unknown`, etc.)
    fn resolve_callee(
        &mut self,
        callee: Option<Box<ast::Expression>>,
    ) -> Result<(ir::Expression, types::FunctionType)> {
        let Some(callee) = callee.and_then(|c| self.visit_callee(*c)) else {
            bail!("");
        };

        if let Some(v) = try_callee_as_variant(&callee) {
            let variant = self.symbols.get(v);
            let e = self.symbols.get(variant.owner);
            let type_params = self.resolve(e.ty).as_enum().unwrap().params.clone();
            let params = variant
                .body
                .iter()
                .map(|m| self.symbol_type_id(*m))
                .collect();
            let f = types::FunctionType {
                type_params,
                params,
                return_type: variant.ty,
            };
            self.intern(f.clone());
            return Ok((callee, f));
        }

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
    fn visit_callee(&mut self, callee: ast::Expression) -> Option<ir::Expression> {
        match callee {
            ast::Expression::Path(p) => self.visit_path_expression(p, super::PathContext::Call),
            expr => self.visit_expression(expr),
        }
    }

    fn check_arguments(
        &mut self,
        args: Vec<ast::Expression>,
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
        node: ast::Expression,
        expected: types::TypeId,
        substitutions: &mut Substitutions,
    ) -> Option<ir::Expression> {
        match node {
            ast::Expression::Function(f) => self
                .check_callback(f, expected, substitutions)
                .map(Into::into),
            expr => self
                .check_expression_against(expr, expected, substitutions)
                .map(|e| e.into()),
        }
    }

    fn check_callback(
        &mut self,
        node: ast::FunctionExpression,
        expected_id: types::TypeId,
        substitutions: &mut Substitutions,
    ) -> Option<ir::FunctionExpression> {
        let expected = self.resolve(expected_id);
        let types::Type::Function(expected) = expected else {
            let expected = self.types.display(expected_id);
            let error = DiagnosticKind::UnexpectedCallback { expected };
            self.error(error, node.loc);
            return None;
        };
        let f = self.visit_function_expression(node, None, Some(&expected))?;
        substitutions.unify(self, expected_id, f.ty, f.loc);
        Some(f)
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
            Some(e) => self.visit_expression(e)?,
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

fn try_callee_as_variant(callee: &ir::Expression) -> Option<VariantSymbolId> {
    let ir::Expression::Identifier(i) = callee else {
        return None;
    };
    match i.symbol {
        SymbolId::Variant(v) => Some(v),
        _ => None,
    }
}
