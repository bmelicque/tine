use crate::{
    ast,
    diagnostics::DiagnosticKind,
    type_checker::analysis_context::type_store::TypeStore,
    types::{ArrayType, TupleType, Type, TypeId},
};

use super::TypeChecker;

impl TypeChecker<'_> {
    pub fn visit_expression(&mut self, node: &ast::Expression) -> TypeId {
        match node {
            ast::Expression::Array(node) => self.visit_array_expression(node),
            ast::Expression::Binary(node) => self.visit_binary_expression(node),
            ast::Expression::BooleanLiteral(_) => TypeStore::BOOLEAN,
            ast::Expression::Block(node) => self.visit_block_expression(node),
            ast::Expression::Call(node) => self.visit_call_expression(node),
            ast::Expression::CompositeLiteral(node) => self.visit_composite_literal(node),
            ast::Expression::Element(node) => self.visit_element_expression(node),
            ast::Expression::Empty => TypeStore::UNIT,
            ast::Expression::FloatLiteral(_) => TypeStore::FLOAT,
            ast::Expression::Member(node) => self.visit_member_expression(node),
            ast::Expression::Function(node) => self.visit_function_expression(node).into(),
            ast::Expression::Identifier(node) => self.visit_identifier(node),
            ast::Expression::If(node) => self.visit_if_expression(node),
            ast::Expression::IfDecl(node) => self.visit_if_decl_expression(node),
            ast::Expression::Invalid(_) => TypeStore::UNKNOWN,
            ast::Expression::IntLiteral(_) => TypeStore::INTEGER,
            ast::Expression::Loop(node) => self.visit_loop(node),
            ast::Expression::Match(node) => self.visit_match_expression(node),
            ast::Expression::StringLiteral(_) => TypeStore::STRING,
            ast::Expression::Tuple(node) => self.visit_tuple_expression(node).into(),
            ast::Expression::Unary(node) => self.visit_unary_expression(&node),
        }
    }

    pub fn visit_expression_or_anonymous(
        &mut self,
        node: &ast::ExpressionOrAnonymous,
        expected_type: TypeId,
    ) -> TypeId {
        match node {
            ast::ExpressionOrAnonymous::Expression(node) => self.visit_expression(node),
            ast::ExpressionOrAnonymous::Struct(node) => self
                .visit_anonymous_struct_literal(node, expected_type)
                .into(),
        }
    }

    fn visit_array_expression(&mut self, node: &ast::ArrayExpression) -> TypeId {
        if node.elements.len() == 0 {
            return self.ctx.save_expression_type(node.loc, TypeStore::DYNAMIC);
        }

        let mut ty = TypeStore::DYNAMIC;
        for value in node.elements.iter() {
            let value_ty = self.visit_expression(value);
            if ty == TypeStore::DYNAMIC {
                ty = value_ty;
                continue;
            }
            self.check_assigned_type(ty, value_ty, value.loc());
        }

        let id = self.intern(Type::Array(ArrayType { element: ty }));
        self.ctx.save_expression_type(node.loc, id)
    }

    pub fn visit_block_expression(&mut self, node: &ast::BlockExpression) -> TypeId {
        // TODO: handle diverging statements (return, break, continue)
        let ty = self.with_scope(|checker| {
            let mut ty = TypeStore::UNIT;
            for stmt in node.statements.iter() {
                ty = checker.visit_statement(&stmt);
            }
            ty
        });
        self.ctx.save_expression_type(node.loc, ty)
    }

    fn visit_identifier(&mut self, node: &ast::Identifier) -> TypeId {
        let var = self.lookup_mut(node.as_str());
        let ty = match var {
            Some(handle) => {
                handle.read(node.loc);
                self.ctx.add_dependencies(vec![handle.readonly()]);
                handle.borrow().get_type()
            }
            None => {
                let error = DiagnosticKind::CannotFindName {
                    name: node.as_str().to_string(),
                };
                self.error(error, node.loc);
                TypeStore::UNKNOWN
            }
        };
        self.ctx.save_expression_type(node.loc, ty)
    }

    fn visit_tuple_expression(&mut self, node: &ast::TupleExpression) -> TypeId {
        let ty = match node.elements.len() {
            0 => TypeStore::UNIT,
            1 => self.visit_expression(&node.elements[0]),
            _ => {
                let ty = Type::Tuple(TupleType {
                    elements: node
                        .elements
                        .iter()
                        .map(|el| self.visit_expression(el))
                        .collect(),
                });
                self.intern(ty)
            }
        };
        self.ctx.save_expression_type(node.loc, ty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::session::Session;
    use crate::ast;
    use crate::locations::Span;
    use crate::types::*;
    use crate::Location;
    use crate::SymbolData;
    use crate::SymbolKind;

    fn create_type_checker() -> TypeChecker<'static> {
        let session = Box::leak(Box::new(Session::new()));
        TypeChecker::new(session, 0)
    }

    fn loc(text: &'static str) -> Location {
        let span = Span::new(0, text.len() as u32);
        Location::new(0, span)
    }

    fn ident(text: &str) -> ast::Identifier {
        ast::Identifier {
            loc: Location::new(0, Span::new(0, text.len() as u32)),
            text: text.to_string(),
        }
    }

    #[test]
    fn test_visit_array_expression_empty() {
        let mut checker = create_type_checker();
        let array_expression = ast::ArrayExpression {
            elements: vec![],
            loc: Location::dummy(),
        };

        let result = checker.visit_array_expression(&array_expression);
        assert_eq!(result, TypeStore::DYNAMIC);
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn test_visit_array_expression_consistent_types() {
        let mut checker = create_type_checker();
        let array_expression = ast::ArrayExpression {
            elements: vec![
                ast::Expression::IntLiteral(ast::IntLiteral {
                    value: 1,
                    loc: Location::dummy(),
                }),
                ast::Expression::IntLiteral(ast::IntLiteral {
                    value: 2,
                    loc: Location::dummy(),
                }),
            ],
            loc: Location::dummy(),
        };

        let result = checker.visit_array_expression(&array_expression);
        let result = checker.resolve(result);
        assert_eq!(
            result,
            Type::Array(ArrayType {
                element: TypeStore::INTEGER
            })
        );
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn test_visit_array_expression_mixed_types() {
        let mut checker = create_type_checker();
        let array_expression = ast::ArrayExpression {
            elements: vec![
                ast::Expression::IntLiteral(ast::IntLiteral {
                    value: 1,
                    loc: Location::dummy(),
                }),
                ast::Expression::StringLiteral(ast::StringLiteral {
                    loc: Location::dummy(),
                    text: "hello".into(),
                }),
            ],
            loc: Location::dummy(),
        };

        let result = checker.visit_array_expression(&array_expression);
        let result = checker.resolve(result);
        assert_eq!(
            result,
            Type::Array(ArrayType {
                element: TypeStore::INTEGER
            })
        );
        assert_eq!(checker.diagnostics.len(), 1);
    }

    #[test]
    fn test_visit_binary_expression() {
        let mut checker = create_type_checker();
        let binary_expression = ast::BinaryExpression {
            left: Box::new(ast::Expression::IntLiteral(ast::IntLiteral {
                value: 1,
                loc: Location::dummy(),
            })),
            right: Box::new(ast::Expression::IntLiteral(ast::IntLiteral {
                value: 2,
                loc: Location::dummy(),
            })),
            operator: ast::BinaryOperator::Add,
            loc: Location::dummy(),
        };

        let result = checker.visit_binary_expression(&binary_expression);
        assert_eq!(result, TypeStore::INTEGER);
        assert!(
            checker.diagnostics.is_empty(),
            "expected no errors, got {:?}",
            checker.diagnostics
        );
    }

    #[test]
    fn test_visit_function_expression() {
        let mut checker = create_type_checker();
        let function_expression = ast::FunctionExpression {
            loc: Location::dummy(),
            name: None,
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
                        left: Box::new(ast::Expression::Identifier(ident("x"))),
                        right: Box::new(ast::Expression::Identifier(ident("y"))),
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
    fn test_visit_identifier() {
        let mut checker = create_type_checker();
        checker.ctx.register_symbol(SymbolData {
            name: "x".into(),
            ty: TypeStore::INTEGER,
            kind: SymbolKind::constant(),
            defined_at: loc("x"),
            ..Default::default()
        });

        let identifier = ident("x");

        let result = checker.visit_identifier(&identifier);
        assert_eq!(result, TypeStore::INTEGER);
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn test_visit_tuple_expression_empty() {
        let mut checker = create_type_checker();
        let tuple_expression = ast::TupleExpression {
            elements: vec![],
            loc: Location::dummy(),
        };

        let result = checker.visit_tuple_expression(&tuple_expression);
        let result = checker.resolve(result);
        assert_eq!(result, Type::Unit);
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn test_visit_tuple_expression_multiple_elements() {
        let mut checker = create_type_checker();
        let tuple_expression = ast::TupleExpression {
            elements: vec![
                ast::Expression::IntLiteral(ast::IntLiteral {
                    value: 42,
                    loc: Location::dummy(),
                }),
                ast::Expression::StringLiteral(ast::StringLiteral {
                    loc: Location::dummy(),
                    text: "".into(),
                }),
                ast::Expression::BooleanLiteral(ast::BooleanLiteral {
                    value: true,
                    loc: Location::dummy(),
                }),
            ],
            loc: Location::dummy(),
        };

        let result = checker.visit_tuple_expression(&tuple_expression);
        let result = checker.resolve(result);
        assert_eq!(
            result,
            Type::Tuple(TupleType {
                elements: vec![TypeStore::INTEGER, TypeStore::STRING, TypeStore::BOOLEAN]
            })
        );
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn test_visit_tuple_expression_nested() {
        let mut checker = create_type_checker();
        let tuple_expression = ast::TupleExpression {
            elements: vec![
                ast::Expression::IntLiteral(ast::IntLiteral {
                    value: 42,
                    loc: Location::dummy(),
                }),
                ast::Expression::Tuple(ast::TupleExpression {
                    elements: vec![
                        ast::Expression::StringLiteral(ast::StringLiteral {
                            loc: Location::dummy(),
                            text: "".into(),
                        }),
                        ast::Expression::BooleanLiteral(ast::BooleanLiteral {
                            value: false,
                            loc: Location::dummy(),
                        }),
                    ],
                    loc: Location::dummy(),
                }),
            ],
            loc: Location::dummy(),
        };

        let result = checker.visit_tuple_expression(&tuple_expression);
        let result = checker.resolve(result);
        assert!(matches!(result, Type::Tuple(_)));
        assert!(checker.diagnostics.is_empty());
    }
}
