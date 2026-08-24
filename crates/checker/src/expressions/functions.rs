use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};
use tine_ir::{self as ir, Typed};
use tine_symbols::symbols::*;
use tine_types::{store::TypeStore, types};

use crate::{substitutions::Substitutions, TypeChecker};

struct FunctionResult {
    pub params: Vec<(Location, VariableSymbolId)>,
    pub return_type: types::TypeId,
    pub body: ir::Block,
}

impl TypeChecker {
    pub fn visit_function_expression(
        &mut self,
        node: ast::FunctionExpression,
        docs: Option<String>,
        type_hint: Option<&types::FunctionType>,
    ) -> Option<ir::FunctionExpression> {
        let mut sub = Substitutions::new();
        let (result, type_params) = self.with_type_params(&node.type_params, |s, _| {
            // TODO: handle generic type_hint
            let params = s.visit_function_params(node.params, type_hint, &mut sub);
            let (return_type, body) =
                s.visit_function_return_body(node.return_type, node.body, type_hint, &mut sub)?;
            let params = params?;
            Some(FunctionResult {
                params,
                return_type,
                body,
            })
        });
        let FunctionResult {
            params,
            return_type,
            body,
        } = result?;

        let ty = self.intern(types::FunctionType {
            type_params,
            params: params.iter().map(|p| self.symbol_type_id(p.1)).collect(),
            return_type,
        });
        let ty = sub.apply(&mut self.types, ty);
        self.errors(sub.produce_diagnostics(&self.types), node.loc);

        let name = match node.name {
            Some(id) => {
                let symbol: FunctionSymbolId = self.symbols.insert(FunctionSymbol {
                    name: id.text.clone(),
                    ty,
                    param_names: params
                        .iter()
                        .map(|p| self.symbol_name(p.1).to_string())
                        .collect(),
                    defined_at: id.loc,
                    docs,
                    ..Default::default()
                });
                self.current_scope().bind(id.text, symbol.into());
                Some((id.loc, symbol))
            }
            None => None,
        };

        Some(ir::FunctionExpression {
            loc: node.loc,
            name,
            params,
            body,
            ty,
        })
    }

    pub fn visit_function_params(
        &mut self,
        node: Option<ast::FunctionParams>,
        hint: Option<&types::FunctionType>,
        sub: &mut Substitutions,
    ) -> Option<Vec<(Location, VariableSymbolId)>> {
        let node = node?;
        let Some(hint) = hint else {
            return node
                .params
                .into_iter()
                .map(|p| Some((p.loc, self.visit_function_param(p, None, sub)?)))
                .collect::<Option<Vec<_>>>();
        };

        self.report_extra_parameters(&node.params, hint, node.loc(), sub);

        node.params
            .into_iter()
            .zip(&hint.params)
            .map(|(param, &hint)| {
                Some((
                    param.loc,
                    self.visit_function_param(param, Some(hint), sub)?,
                ))
            })
            .collect::<Option<Vec<_>>>()
    }

    fn report_extra_parameters(
        &mut self,
        params: &[ast::FunctionParam],
        hint: &types::FunctionType,
        error_loc: Location,
        sub: &mut Substitutions,
    ) {
        if hint.params.len() == params.len() {
            return;
        }
        let expected = hint.params.len();
        let got = params.len();
        let error = DiagnosticKind::ArgumentCountMismatch { expected, got };
        self.error(error, error_loc);
        params
            .iter()
            .skip(hint.params.len())
            .map(|p| self.visit_function_param(p.clone(), Some(TypeStore::UNKNOWN), sub))
            .for_each(drop);
    }

    fn visit_function_param(
        &mut self,
        node: ast::FunctionParam,
        hint: Option<types::TypeId>,
        sub: &mut Substitutions,
    ) -> Option<VariableSymbolId> {
        let name = node.name?;
        let ty = match (hint, node.type_annotation) {
            (Some(hint), Some(ty)) => {
                let loc = ty.loc();
                let ty = self.visit_type(ty);
                self.try_unify(ty, hint, sub, loc);
                ty
            }
            (Some(hint), None) => hint,
            (None, Some(ty)) => self.visit_type(ty),
            (None, None) => {
                self.error(DiagnosticKind::CannotInferType, node.loc);
                TypeStore::UNKNOWN
            }
        };
        let id = self.symbols.insert::<VariableSymbolId>(VariableSymbol {
            name: name.as_str().into(),
            ty,
            defined_at: name.loc,
            ..Default::default()
        });
        self.current_scope().bind(name.text, id.into());
        Some(id)
    }

    /// Return (function return type, visited body)
    pub fn visit_function_return_body(
        &mut self,
        return_annotation: Option<ast::Type>,
        body: Option<Box<ast::Expression>>,
        hint: Option<&types::FunctionType>,
        sub: &mut Substitutions,
    ) -> Option<(types::TypeId, ir::Block)> {
        let body_must_be_block = hint.is_none() || return_annotation.is_some();
        let return_type = self.visit_return_type(return_annotation, hint, sub);
        let mut body = self.visit_function_body(*body?, body_must_be_block)?;
        match hint {
            Some(_) => self.check_callback_body_type(&body, return_type, sub),
            None => self.check_function_body_type(&body, return_type),
        }
        return_last(&mut body);
        Some((return_type, body))
    }

    pub fn check_function_body_type(&mut self, body: &ir::Block, return_type: types::TypeId) {
        for ret in body.find_returns() {
            let ty = ret.expression.as_ref().map_or(TypeStore::UNIT, |e| e.ty());
            self.check_assigned_type(return_type, ty, false, ret.loc);
        }

        if return_type != TypeStore::UNIT {
            let loc = match body.statements.last() {
                Some(stmt) => stmt.loc(),
                None => body.loc,
            };
            self.check_assigned_type(return_type, body.ty, false, loc);
        }
    }
    pub fn check_callback_body_type(
        &mut self,
        body: &ir::Block,
        return_type: types::TypeId,
        sub: &mut Substitutions,
    ) {
        for ret in body.find_returns() {
            let ty = ret.expression.as_ref().map_or(TypeStore::UNIT, |e| e.ty());
            self.try_unify(ty, return_type, sub, ret.loc);
        }

        if return_type != TypeStore::UNIT {
            let loc = match body.statements.last() {
                Some(stmt) => stmt.loc(),
                None => body.loc,
            };
            self.try_unify(body.ty, return_type, sub, loc);
        }
    }

    fn try_unify(
        &mut self,
        ty: types::TypeId,
        hint: types::TypeId,
        sub: &mut Substitutions,
        loc: Location,
    ) {
        match self.resolve(hint) {
            types::Type::Param(p) => {
                sub.unify(self, p.id, ty, loc);
            }
            _ => {
                self.check_assigned_type(hint, ty, true, loc);
            }
        }
    }

    fn visit_return_type(
        &mut self,
        return_type: Option<ast::Type>,
        hint: Option<&types::FunctionType>,
        sub: &mut Substitutions,
    ) -> types::TypeId {
        let Some(hint) = hint else {
            return return_type.map_or(TypeStore::UNIT, |ty| self.visit_type(ty));
        };
        if let Some(return_type) = return_type {
            let return_loc = return_type.loc();
            let ty = self.visit_type(return_type);
            self.try_unify(ty, hint.return_type, sub, return_loc);
        }
        sub.apply(&mut self.types, hint.return_type)
    }

    fn visit_function_body(
        &mut self,
        body: ast::Expression,
        must_be_block: bool,
    ) -> Option<ir::Block> {
        let body = self.visit_expression(body)?;
        if must_be_block && body.as_block().is_none() {
            self.error(DiagnosticKind::ExpectedBlock, body.loc());
            return None;
        }
        Some(body.into())
    }
}

fn return_last(body: &mut ir::Block) {
    let Some(last) = body.statements.pop() else {
        return;
    };
    let last = match last {
        ir::Statement::Expression(e) => ir::Statement::Return(ir::ReturnStatement {
            loc: e.loc(),
            expression: Some(Box::new(e)),
        }),
        _ => last,
    };
    body.statements.push(last);
}

#[cfg(test)]
mod tests {
    use tine_common::locations::Span;

    use super::*;

    fn ident(text: &str) -> ast::Identifier {
        ast::Identifier {
            loc: Location::new(0, Span::new(0, text.len() as u32)),
            text: text.to_string(),
        }
    }

    #[test]
    fn test_visit_function_expression() {
        let mut checker = TypeChecker::new();
        let function_expression = ast::FunctionExpression {
            loc: Location::dummy(),
            name: None,
            type_params: None,
            params: Some(ast::FunctionParams {
                loc: Location::dummy(),
                params: vec![
                    ast::FunctionParam {
                        name: Some(ident("x")),
                        type_annotation: Some(ast::Type::Named(ast::NamedType {
                            name: ast::Identifier {
                                text: "int".to_string(),
                                ..Default::default()
                            },
                            ..Default::default()
                        })),
                        loc: Location::dummy(),
                    },
                    ast::FunctionParam {
                        name: Some(ident("y")),
                        type_annotation: Some(ast::Type::Named(ast::NamedType {
                            name: ast::Identifier {
                                text: "int".to_string(),
                                ..Default::default()
                            },
                            ..Default::default()
                        })),
                        loc: Location::dummy(),
                    },
                ],
            }),
            return_type: Some(ast::Type::Named(ast::NamedType {
                name: ast::Identifier {
                    text: "int".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            })),
            body: Some(Box::new(ast::Expression::Block(ast::BlockExpression {
                loc: Location::dummy(),
                statements: vec![ast::Statement::Expression(ast::ExpressionStatement {
                    expression: Box::new(ast::Expression::Binary(ast::BinaryExpression {
                        left: Some(Box::new(ast::Expression::Identifier(ident("x")))),
                        right: Some(Box::new(ast::Expression::Identifier(ident("y")))),
                        operator: ast::BinaryOperator::Add,
                        loc: Location::dummy(),
                    })),
                })],
            }))),
        };

        let result = checker.visit_function_expression(function_expression, None, None);
        let result = checker.resolve(result.map_or(TypeStore::UNKNOWN, |r| r.ty));
        assert_eq!(
            result,
            types::Type::Function(types::FunctionType {
                params: vec![TypeStore::INTEGER, TypeStore::INTEGER],
                return_type: TypeStore::INTEGER,
                ..Default::default()
            })
        );
        assert!(
            checker.diagnostics.is_empty(),
            "expected no error, found {:?}",
            checker.diagnostics
        );
    }

    #[test]
    fn test_visit_generic_function_expression() {
        let mut checker = TypeChecker::new();
        let function_expression = ast::FunctionExpression {
            type_params: Some(vec![ast::Identifier {
                text: "T".to_string(),
                loc: Location::dummy(),
            }]),
            params: Some(ast::FunctionParams {
                loc: Location::dummy(),
                params: vec![ast::FunctionParam {
                    name: Some(ident("x")),
                    type_annotation: Some(ast::Type::Named(ast::NamedType {
                        name: ast::Identifier {
                            text: "T".to_string(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })),
                    loc: Location::dummy(),
                }],
            }),
            body: Some(Box::new(ast::BlockExpression::default().into())),
            ..Default::default()
        };

        let result = checker.visit_function_expression(function_expression, None, None);
        assert!(
            checker.diagnostics.is_empty(),
            "expected no errors, got {:?}",
            checker.diagnostics
        );

        let result = checker.resolve(result.map_or(TypeStore::UNKNOWN, |r| r.ty));

        let types::Type::Function(f) = result else {
            panic!("function expected")
        };
        assert_eq!(f.type_params[0].name, "T");
        assert!(matches!(
            checker.resolve(f.params[0]),
            types::Type::Param(_)
        ));
        assert_eq!(f.return_type, TypeStore::UNIT);
    }
}
