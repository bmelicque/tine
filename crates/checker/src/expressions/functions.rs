use tine_ast as ast;
use tine_common::locations::Location;
use tine_ir::{self as ir, Typed};
use tine_symbols::symbols::*;
use tine_types::{store::TypeStore, types};

use crate::TypeChecker;

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
    ) -> Option<ir::FunctionExpression> {
        let (result, type_params) = self.with_type_params(&node.type_params, |s| {
            let params = s.visit_function_params(node.params);
            let (return_type, body) = s.visit_function_body(node.return_type, node.body)?;
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
    ) -> Option<Vec<(Location, VariableSymbolId)>> {
        let node = node?;
        node.params
            .into_iter()
            .map(|p| Some((p.loc, self.visit_function_param(p)?)))
            .collect::<Option<Vec<_>>>()
    }

    fn visit_function_param(&mut self, node: ast::FunctionParam) -> Option<VariableSymbolId> {
        let name = node.name?;
        let ty = node
            .type_annotation
            .map_or(TypeStore::UNKNOWN, |t| self.visit_type(t));
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
    pub fn visit_function_body(
        &mut self,
        return_type: Option<ast::Type>,
        body: Option<ast::BlockExpression>,
    ) -> Option<(types::TypeId, ir::Block)> {
        let return_type = return_type.map_or(TypeStore::UNIT, |ty| self.visit_type(ty));

        let body = body.map(|b| self.visit_block_expression(b))?;

        for ret in body.find_returns() {
            let ty = ret.expression.as_ref().map_or(TypeStore::UNIT, |e| e.ty());
            self.check_assigned_type(return_type, ty, false, ret.loc);
        }

        if return_type != TypeStore::UNIT {
            self.check_assigned_type(return_type, body.ty, false, body.loc);
        }

        Some((return_type, body))
    }
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
            body: Some(ast::BlockExpression {
                loc: Location::dummy(),
                statements: vec![ast::Statement::Expression(ast::ExpressionStatement {
                    expression: Box::new(ast::Expression::Binary(ast::BinaryExpression {
                        left: Some(Box::new(ast::Expression::Identifier(ident("x")))),
                        right: Some(Box::new(ast::Expression::Identifier(ident("y")))),
                        operator: ast::BinaryOperator::Add,
                        loc: Location::dummy(),
                    })),
                })],
            }),
        };

        let result = checker.visit_function_expression(function_expression, None);
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
            body: Some(ast::BlockExpression::default()),
            ..Default::default()
        };

        let result = checker.visit_function_expression(function_expression, None);
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
