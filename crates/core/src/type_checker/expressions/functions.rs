use crate::{
    ast,
    type_checker::{analysis_context::type_store::TypeStore, TypeChecker},
    types::{FunctionType, GenericType, Type, TypeId},
    SymbolData, SymbolKind,
};

impl TypeChecker<'_> {
    pub fn visit_function_expression(&mut self, node: &ast::FunctionExpression) -> TypeId {
        let ((params, return_type), type_params) = self.with_type_params(&node.type_params, |s| {
            let param_types = s.visit_function_params(&node);
            let body_type = s.visit_function_body(node);
            (param_types, body_type)
        });

        let ty = self.intern(Type::Function(FunctionType {
            params,
            return_type,
        }));
        let ty = match type_params.len() {
            0 => ty,
            _ => self.intern(Type::Generic(GenericType {
                params: type_params,
                definition: ty,
            })),
        };

        self.ctx.save_expression_type(node.loc, ty)
    }

    fn visit_function_params(&mut self, node: &ast::FunctionExpression) -> Vec<TypeId> {
        let mut param_types = Vec::with_capacity(node.params.len());
        for param in node.params.iter() {
            let ty = param
                .type_annotation
                .as_ref()
                .map(|t| self.visit_type(t))
                .unwrap_or(TypeStore::UNKNOWN);
            self.ctx.register_symbol(SymbolData {
                name: param.name.as_str().into(),
                ty,
                kind: SymbolKind::constant(),
                defined_at: param.name.loc,
                ..Default::default()
            });
            param_types.push(ty);
        }
        param_types
    }

    pub fn visit_function_body(&mut self, node: &ast::FunctionExpression) -> TypeId {
        let return_type = node
            .return_type
            .as_ref()
            .map(|ty| self.visit_type(ty))
            .unwrap_or(TypeStore::UNIT);

        let body_type = self.visit_block_expression(&node.body);
        let mut returns = Vec::<ast::ReturnStatement>::new();
        node.body.find_returns(&mut returns);
        for ret in returns {
            let ty = match ret.value {
                Some(value) => self.get_type_at(value.loc()).unwrap_or(TypeStore::UNKNOWN),
                None => TypeStore::UNIT,
            };
            self.check_assigned_type(return_type, ty, ret.loc);
        }

        if let Some(ast::Statement::Expression(expr)) = node.body.statements.last() {
            if return_type != TypeStore::UNIT {
                self.check_assigned_type(return_type, body_type, expr.expression.loc());
            }
        }

        return_type
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
            params: vec![
                ast::FunctionParam {
                    name: ident("x"),
                    type_annotation: Some(ast::Type::Named(ast::NamedType {
                        name: "int".to_string(),
                        args: None,
                        loc: Location::dummy(),
                    })),
                    loc: Location::dummy(),
                },
                ast::FunctionParam {
                    name: ident("y"),
                    type_annotation: Some(ast::Type::Named(ast::NamedType {
                        name: "int".to_string(),
                        args: None,
                        loc: Location::dummy(),
                    })),
                    loc: Location::dummy(),
                },
            ],
            return_type: Some(ast::Type::Named(ast::NamedType {
                loc: Location::dummy(),
                name: "int".into(),
                args: None,
            })),
            body: ast::BlockExpression {
                loc: Location::dummy(),
                statements: vec![ast::Statement::Expression(ast::ExpressionStatement {
                    expression: Box::new(ast::Expression::Binary(ast::BinaryExpression {
                        left: Some(Box::new(ast::Expression::Identifier(ident("x")))),
                        right: Some(Box::new(ast::Expression::Identifier(ident("y")))),
                        operator: ast::BinaryOperator::Add,
                        loc: Location::dummy(),
                    })),
                })],
            },
        };

        let result = checker.visit_function_expression(&function_expression);
        let result = checker.resolve(result);
        assert_eq!(
            result,
            Type::Function(FunctionType {
                params: vec![TypeStore::INTEGER, TypeStore::INTEGER],
                return_type: TypeStore::INTEGER,
            })
        );
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn test_visit_generic_function_expression() {
        let mut checker = create_type_checker();
        let function_expression = ast::FunctionExpression {
            loc: Location::dummy(),
            name: None,
            type_params: Some(vec![ast::Identifier {
                text: "T".to_string(),
                loc: Location::dummy(),
            }]),
            params: vec![ast::FunctionParam {
                name: ident("x"),
                type_annotation: Some(ast::Type::Named(ast::NamedType {
                    name: "T".to_string(),
                    args: None,
                    loc: Location::dummy(),
                })),
                loc: Location::dummy(),
            }],
            return_type: None,
            body: ast::BlockExpression {
                loc: Location::dummy(),
                statements: vec![],
            },
        };

        let result = checker.visit_function_expression(&function_expression);
        assert!(checker.diagnostics.is_empty());

        let result = checker.resolve(result);

        let Type::Generic(GenericType { params, definition }) = result else {
            panic!("Expected generic type");
        };

        assert_eq!(params.len(), 1);
        let param = checker.resolve(params[0]);
        assert_eq!(
            param,
            Type::Param(TypeParam {
                name: "T".to_string(),
                idx: 0
            })
        );

        assert_eq!(
            checker.resolve(definition),
            Type::Function(FunctionType {
                params: vec![params[0]],
                return_type: TypeStore::UNIT,
            })
        );
    }
}
