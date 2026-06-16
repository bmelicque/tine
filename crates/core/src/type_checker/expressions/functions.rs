use crate::{
    ast, ir,
    type_checker::{analysis_context::type_store::TypeStore, TypeChecker},
    types::{FunctionType, TypeId},
    SymbolData, SymbolKind,
};

struct FunctionResult {
    pub params: Vec<ir::Identifier>,
    pub return_type: TypeId,
    pub body: ir::Block,
}

impl TypeChecker<'_> {
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

        let ty = self.intern(FunctionType {
            type_params,
            params: params.iter().map(|p| p.ty()).collect(),
            return_type,
        });

        let name = match node.name {
            Some(id) => {
                let symbol = self.ctx.register_symbol(SymbolData {
                    name: id.text,
                    ty,
                    kind: SymbolKind::Function {
                        param_names: params.iter().map(|p| p.as_name()).collect(),
                    },
                    defined_at: id.loc,
                    docs,
                    ..Default::default()
                });
                Some(ir::Identifier {
                    loc: id.loc,
                    symbol,
                })
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
    ) -> Option<Vec<ir::Identifier>> {
        let node = node?;
        node.params
            .into_iter()
            .map(|p| self.visit_function_param(p))
            .collect::<Option<Vec<_>>>()
    }

    fn visit_function_param(&mut self, node: ast::FunctionParam) -> Option<ir::Identifier> {
        let name = node.name?;
        let ty = node
            .type_annotation
            .map_or(TypeStore::UNKNOWN, |t| self.visit_type(t));
        let symbol = self.ctx.register_symbol(SymbolData {
            name: name.as_str().into(),
            ty,
            kind: SymbolKind::constant(),
            defined_at: name.loc,
            ..Default::default()
        });
        Some(ir::Identifier {
            loc: name.loc,
            symbol,
        })
    }

    /// Return (function return type, visited body)
    pub fn visit_function_body(
        &mut self,
        return_type: Option<ast::Type>,
        body: Option<ast::BlockExpression>,
    ) -> Option<(TypeId, ir::Block)> {
        let return_type = return_type.map_or(TypeStore::UNIT, |ty| self.visit_type(ty));

        let body = body.map(|b| self.visit_block_expression(b))?;

        for ret in body.find_returns() {
            let ty = ret.expression.as_ref().map_or(TypeStore::UNIT, |e| e.ty());
            self.check_assigned_type(return_type, ty, ret.loc);
        }

        if return_type != TypeStore::UNIT {
            self.check_assigned_type(return_type, body.ty, body.loc);
        }

        Some((return_type, body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::session::Session;
    use crate::ast;
    use crate::locations::Span;
    use crate::type_checker::test_utils::MockLoader;
    use crate::types::*;
    use crate::Location;

    fn create_type_checker() -> TypeChecker<'static> {
        let session = Box::leak(Box::new(Session::new(Box::new(MockLoader))));
        TypeChecker::new(session, 0)
    }

    fn ident(text: &str) -> ast::Identifier {
        ast::Identifier {
            loc: Location::new(0, Span::new(0, text.len() as u32)),
            text: text.to_string(),
        }
    }

    #[test]
    fn test_visit_function_expression() {
        let mut checker = create_type_checker();
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
                            name: "int".to_string(),
                            ..Default::default()
                        })),
                        loc: Location::dummy(),
                    },
                    ast::FunctionParam {
                        name: Some(ident("y")),
                        type_annotation: Some(ast::Type::Named(ast::NamedType {
                            name: "int".to_string(),
                            ..Default::default()
                        })),
                        loc: Location::dummy(),
                    },
                ],
            }),
            return_type: Some(ast::Type::Named(ast::NamedType {
                name: "int".into(),
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
            Type::Function(FunctionType {
                params: vec![TypeStore::INTEGER, TypeStore::INTEGER],
                return_type: TypeStore::INTEGER,
                ..Default::default()
            })
        );
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn test_visit_generic_function_expression() {
        let mut checker = create_type_checker();
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
                        name: "T".to_string(),
                        ..Default::default()
                    })),
                    loc: Location::dummy(),
                }],
            }),
            body: Some(ast::BlockExpression::default()),
            ..Default::default()
        };

        let result = checker.visit_function_expression(function_expression, None);
        assert!(checker.diagnostics.is_empty());

        let result = checker.resolve(result.map_or(TypeStore::UNKNOWN, |r| r.ty));

        let Type::Function(f) = result else {
            panic!("function expected")
        };
        assert_eq!(f.type_params[0].name, "T");
        assert!(matches!(checker.resolve(f.params[0]), Type::Param(_)));
        assert_eq!(f.return_type, TypeStore::UNIT);
    }
}
