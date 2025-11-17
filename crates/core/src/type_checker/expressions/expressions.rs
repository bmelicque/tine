use crate::{
    ast,
    type_checker::analysis_context::type_store::TypeStore,
    types::{ArrayType, TupleType, Type, TypeId},
};

use super::TypeChecker;

impl TypeChecker {
    pub fn visit_expression(&mut self, node: &ast::Expression) -> TypeId {
        match node {
            ast::Expression::Array(node) => self.visit_array_expression(node),
            ast::Expression::Binary(node) => self.visit_binary_expression(node),
            ast::Expression::BooleanLiteral(_) => TypeStore::BOOLEAN,
            ast::Expression::Block(node) => self.visit_block_expression(node),
            ast::Expression::Call(node) => self.visit_call_expression(node),
            ast::Expression::CompositeLiteral(node) => self.visit_composite_literal(node),
            ast::Expression::Element(node) => self.visit_element_expression(node),
            ast::Expression::Empty => TypeStore::VOID,
            ast::Expression::Member(node) => self.visit_member_expression(node),
            ast::Expression::Function(node) => self.visit_function_expression(node).into(),
            ast::Expression::Identifier(node) => self.visit_identifier(node),
            ast::Expression::If(node) => self.visit_if_expression(node),
            ast::Expression::IfDecl(node) => self.visit_if_decl_expression(node),
            ast::Expression::Invalid(_) => TypeStore::UNKNOWN,
            ast::Expression::Loop(node) => self.visit_loop(node),
            ast::Expression::Match(node) => self.visit_match_expression(node),
            ast::Expression::NumberLiteral(_) => TypeStore::NUMBER,
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
            return self
                .analysis_context
                .save_expression_type(node.span, TypeStore::DYNAMIC);
        }

        let mut ty = TypeStore::DYNAMIC;
        for value in node.elements.iter() {
            let value_ty = self.visit_expression(value);
            if ty == TypeStore::DYNAMIC {
                ty = value_ty;
                continue;
            }
            self.check_assigned_type(ty, value_ty, value.as_span());
        }

        let id = self
            .analysis_context
            .type_store
            .add(Type::Array(ArrayType { element: ty }));
        self.analysis_context.save_expression_type(node.span, id)
    }

    pub fn visit_block_expression(&mut self, node: &ast::BlockExpression) -> TypeId {
        // TODO: handle diverging statements (return, break, continue)
        let ty = self.with_scope(node.span, |checker| {
            let mut ty = TypeStore::VOID;
            for stmt in node.statements.iter() {
                ty = checker.visit_statement(&stmt);
            }
            ty
        });
        self.analysis_context.save_expression_type(node.span, ty)
    }

    fn visit_identifier(&mut self, node: &ast::Identifier) -> TypeId {
        let var = self.analysis_context.lookup_mut(node.as_str());
        let ty = match var {
            Some(handle) => {
                handle.add_read();
                self.analysis_context
                    .add_dependencies(vec![handle.readonly()]);
                self.analysis_context
                    .save_symbol_token(node.span, handle.readonly());
                handle.borrow().ty.clone()
            }
            None => {
                self.error(format!("Undefined variable: {}", node.as_str()), node.span);
                TypeStore::UNKNOWN
            }
        };
        self.analysis_context.save_expression_type(node.span, ty)
    }

    fn visit_tuple_expression(&mut self, node: &ast::TupleExpression) -> TypeId {
        let ty = TupleType {
            elements: node
                .elements
                .iter()
                .map(|el| self.visit_expression(el))
                .collect(),
        };
        let ty = self.analysis_context.type_store.add(ty.into());
        self.analysis_context.save_expression_type(node.span, ty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;
    use crate::types::*;
    use crate::VariableData;

    fn create_type_checker() -> TypeChecker {
        TypeChecker::new(Vec::new())
    }

    fn dummy_span() -> pest::Span<'static> {
        pest::Span::new("_", 0, 0).unwrap()
    }

    fn span(text: &'static str) -> pest::Span<'static> {
        pest::Span::new(text, 0, text.len()).unwrap()
    }

    #[test]
    fn test_visit_array_expression_empty() {
        let mut checker = create_type_checker();
        let array_expression = ast::ArrayExpression {
            elements: vec![],
            span: dummy_span(),
        };

        let result = checker.visit_array_expression(&array_expression);
        assert_eq!(result, TypeStore::DYNAMIC);
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_array_expression_consistent_types() {
        let mut checker = create_type_checker();
        let array_expression = ast::ArrayExpression {
            elements: vec![
                ast::Expression::NumberLiteral(ast::NumberLiteral {
                    value: ordered_float::OrderedFloat(1.0),
                    span: dummy_span(),
                }),
                ast::Expression::NumberLiteral(ast::NumberLiteral {
                    value: ordered_float::OrderedFloat(2.0),
                    span: dummy_span(),
                }),
                ast::Expression::NumberLiteral(ast::NumberLiteral {
                    value: ordered_float::OrderedFloat(3.0),
                    span: dummy_span(),
                }),
            ],
            span: dummy_span(),
        };

        let result = checker.visit_array_expression(&array_expression);
        let result = checker.resolve(result);
        assert_eq!(
            *result,
            Type::Array(ArrayType {
                element: TypeStore::NUMBER
            })
        );
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_array_expression_mixed_types() {
        let mut checker = create_type_checker();
        let array_expression = ast::ArrayExpression {
            elements: vec![
                ast::Expression::NumberLiteral(ast::NumberLiteral {
                    value: ordered_float::OrderedFloat(1.0),
                    span: dummy_span(),
                }),
                ast::Expression::StringLiteral(ast::StringLiteral {
                    span: span("hello"),
                }),
            ],
            span: dummy_span(),
        };

        let result = checker.visit_array_expression(&array_expression);
        let result = checker.resolve(result);
        assert_eq!(
            *result,
            Type::Array(ArrayType {
                element: TypeStore::NUMBER
            })
        );
        assert_eq!(checker.errors.len(), 1);
    }

    #[test]
    fn test_visit_binary_expression() {
        let mut checker = create_type_checker();
        let binary_expression = ast::BinaryExpression {
            left: Box::new(ast::Expression::NumberLiteral(ast::NumberLiteral {
                value: ordered_float::OrderedFloat(1.0),
                span: dummy_span(),
            })),
            right: Box::new(ast::Expression::NumberLiteral(ast::NumberLiteral {
                value: ordered_float::OrderedFloat(2.0),
                span: dummy_span(),
            })),
            operator: ast::BinaryOperator::Add,
            span: dummy_span(),
        };

        let result = checker.visit_binary_expression(&binary_expression);
        assert_eq!(result, TypeStore::NUMBER);
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_function_expression() {
        let mut checker = create_type_checker();
        let function_expression = ast::FunctionExpression {
            params: vec![
                ast::FunctionParam {
                    name: ast::Identifier { span: span("x") },
                    type_annotation: ast::Type::Named(ast::NamedType {
                        name: "number".to_string(),
                        args: None,
                        span: dummy_span(),
                    }),
                    span: dummy_span(),
                },
                ast::FunctionParam {
                    name: ast::Identifier { span: span("y") },
                    type_annotation: ast::Type::Named(ast::NamedType {
                        name: "number".to_string(),
                        args: None,
                        span: dummy_span(),
                    }),
                    span: dummy_span(),
                },
            ],
            body: ast::FunctionBody::Expression(Box::new(ast::Expression::Binary(
                ast::BinaryExpression {
                    left: Box::new(ast::Expression::Identifier(ast::Identifier {
                        span: span("x"),
                    })),
                    right: Box::new(ast::Expression::Identifier(ast::Identifier {
                        span: span("y"),
                    })),
                    operator: ast::BinaryOperator::Add,
                    span: dummy_span(),
                },
            ))),
            span: dummy_span(),
        };

        let result = checker.visit_function_expression(&function_expression);
        let result = checker.resolve(result);
        assert_eq!(
            *result,
            Type::Function(FunctionType {
                params: vec![TypeStore::NUMBER, TypeStore::NUMBER],
                return_type: TypeStore::NUMBER,
            })
        );
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_identifier() {
        let mut checker = create_type_checker();
        checker.analysis_context.register_symbol(VariableData::pure(
            "x".into(),
            TypeStore::NUMBER,
            span("x"),
        ));

        let identifier = ast::Identifier { span: span("x") };

        let result = checker.visit_identifier(&identifier);
        assert_eq!(result, TypeStore::NUMBER);
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_tuple_expression_empty() {
        let mut checker = create_type_checker();
        let tuple_expression = ast::TupleExpression {
            elements: vec![],
            span: dummy_span(),
        };

        let result = checker.visit_tuple_expression(&tuple_expression);
        let result = checker.resolve(result);
        assert_eq!(*result, Type::Tuple(TupleType { elements: vec![] }));
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_tuple_expression_multiple_elements() {
        let mut checker = create_type_checker();
        let tuple_expression = ast::TupleExpression {
            elements: vec![
                ast::Expression::NumberLiteral(ast::NumberLiteral {
                    value: ordered_float::OrderedFloat(42.0),
                    span: dummy_span(),
                }),
                ast::Expression::StringLiteral(ast::StringLiteral { span: dummy_span() }),
                ast::Expression::BooleanLiteral(ast::BooleanLiteral {
                    value: true,
                    span: dummy_span(),
                }),
            ],
            span: dummy_span(),
        };

        let result = checker.visit_tuple_expression(&tuple_expression);
        let result = checker.resolve(result);
        assert_eq!(
            *result,
            Type::Tuple(TupleType {
                elements: vec![TypeStore::NUMBER, TypeStore::STRING, TypeStore::BOOLEAN]
            })
        );
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_tuple_expression_nested() {
        let mut checker = create_type_checker();
        let tuple_expression = ast::TupleExpression {
            elements: vec![
                ast::Expression::NumberLiteral(ast::NumberLiteral {
                    value: ordered_float::OrderedFloat(42.0),
                    span: dummy_span(),
                }),
                ast::Expression::Tuple(ast::TupleExpression {
                    elements: vec![
                        ast::Expression::StringLiteral(ast::StringLiteral { span: dummy_span() }),
                        ast::Expression::BooleanLiteral(ast::BooleanLiteral {
                            value: false,
                            span: dummy_span(),
                        }),
                    ],
                    span: dummy_span(),
                }),
            ],
            span: dummy_span(),
        };

        let result = checker.visit_tuple_expression(&tuple_expression);
        let result = checker.resolve(result);
        assert!(matches!(result, Type::Tuple(_)));
        assert!(checker.errors.is_empty());
    }
}
