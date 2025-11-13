use crate::{ast, parser::parser::ParseError, type_checker::analysis_context::VariableData, types};

use super::TypeChecker;

impl TypeChecker {
    pub fn visit_expression(&mut self, node: &ast::Expression) -> types::Type {
        match node {
            ast::Expression::Array(node) => self.visit_array_expression(node),
            ast::Expression::Binary(node) => self.visit_binary_expression(node),
            ast::Expression::BooleanLiteral(_) => types::Type::Boolean,
            ast::Expression::Block(node) => self.visit_block_expression(node),
            ast::Expression::Call(node) => self.visit_call_expression(node),
            ast::Expression::CompositeLiteral(node) => self.visit_composite_literal(node),
            ast::Expression::Element(node) => self.visit_element_expression(node),
            ast::Expression::Empty => types::Type::Void,
            ast::Expression::Member(node) => self.visit_member_expression(node),
            ast::Expression::Function(node) => self.visit_function_expression(node).into(),
            ast::Expression::Identifier(node) => self.visit_identifier(node),
            ast::Expression::If(node) => self.visit_if_expression(node),
            ast::Expression::IfDecl(node) => self.visit_if_decl_expression(node),
            ast::Expression::Invalid(_) => types::Type::Unknown,
            ast::Expression::Loop(node) => self.visit_loop(node),
            ast::Expression::Match(node) => self.visit_match_expression(node),
            ast::Expression::NumberLiteral(_) => types::Type::Number,
            ast::Expression::StringLiteral(_) => types::Type::String,
            ast::Expression::Tuple(node) => self.visit_tuple_expression(node).into(),
            ast::Expression::Unary(node) => self.visit_unary_expression(&node),
        }
    }

    pub fn visit_expression_or_anonymous(
        &mut self,
        node: &ast::ExpressionOrAnonymous,
    ) -> types::Type {
        match node {
            ast::ExpressionOrAnonymous::Expression(node) => self.visit_expression(node),
            ast::ExpressionOrAnonymous::Struct(node) => {
                self.visit_anonymous_struct_literal(node).into()
            }
        }
    }

    fn visit_array_expression(&mut self, node: &ast::ArrayExpression) -> types::Type {
        if node.elements.len() == 0 {
            return self.set_type_at(node.span, types::Type::Dynamic);
        }

        let mut ty = types::Type::Dynamic;
        for value in node.elements.iter() {
            let value_ty = self.visit_expression(value);
            if ty == types::Type::Dynamic {
                ty = value_ty;
                continue;
            }
            if !self.can_be_assigned_to(&value_ty, &ty) {
                self.errors.push(ParseError {
                    message: format!("Type mismatch: expected {}, found {}", ty, value_ty),
                    span: value.as_span(),
                });
                ty = types::Type::Unknown;
            }
        }

        self.set_type_at(
            node.span,
            types::ArrayType {
                element: Box::new(ty),
            }
            .into(),
        )
    }

    fn visit_binary_expression(&mut self, node: &ast::BinaryExpression) -> types::Type {
        let left_type = self.visit_expression(&node.left);
        let right_type = self.visit_expression(&node.right);

        let mut push_error = |ty: types::Type| {
            self.errors.push(ParseError {
                message: format!(
                    "Operator '{}' cannot be applied to type {:?}",
                    node.operator, ty
                ),
                span: node.span,
            })
        };

        match node.operator {
            ast::BinaryOperator::Add
            | ast::BinaryOperator::Sub
            | ast::BinaryOperator::Mul
            | ast::BinaryOperator::Div
            | ast::BinaryOperator::Mod
            | ast::BinaryOperator::Pow
            | ast::BinaryOperator::Geq
            | ast::BinaryOperator::Grt
            | ast::BinaryOperator::Leq
            | ast::BinaryOperator::Less => {
                if !matches!(left_type, types::Type::Unknown | types::Type::Number) {
                    push_error(left_type);
                };
                if !matches!(right_type, types::Type::Unknown | types::Type::Number) {
                    push_error(right_type);
                };
            }
            ast::BinaryOperator::EqEq | ast::BinaryOperator::Neq => {
                if left_type != right_type
                    && left_type != types::Type::Unknown
                    && right_type != types::Type::Unknown
                {
                    self.errors.push(ParseError {
                        message: format!(
                            "Types {:?} and {:?} cannot be compared",
                            left_type, right_type
                        ),
                        span: node.span,
                    });
                    return types::Type::Unknown;
                }
            }
            ast::BinaryOperator::LAnd | ast::BinaryOperator::LOr => {
                if !matches!(left_type, types::Type::Unknown | types::Type::Boolean) {
                    push_error(left_type);
                };
                if !matches!(right_type, types::Type::Unknown | types::Type::Boolean) {
                    push_error(right_type);
                };
            }
        };

        self.set_type_at(
            node.span,
            match node.operator {
                ast::BinaryOperator::Add
                | ast::BinaryOperator::Sub
                | ast::BinaryOperator::Mul
                | ast::BinaryOperator::Div
                | ast::BinaryOperator::Mod
                | ast::BinaryOperator::Pow => types::Type::Number,
                ast::BinaryOperator::EqEq
                | ast::BinaryOperator::Geq
                | ast::BinaryOperator::Grt
                | ast::BinaryOperator::LAnd
                | ast::BinaryOperator::Leq
                | ast::BinaryOperator::Less
                | ast::BinaryOperator::LOr
                | ast::BinaryOperator::Neq => types::Type::Boolean,
            },
        )
    }

    pub fn visit_block_expression(&mut self, node: &ast::BlockExpression) -> types::Type {
        // TODO: handle diverging statements (return, break, continue)
        let ty = self.with_scope(node.span, |checker| {
            let mut ty = types::Type::Void;
            for stmt in node.statements.iter() {
                ty = checker.visit_statement(&stmt);
            }
            ty
        });
        self.set_type_at(node.span, ty)
    }

    pub fn visit_function_expression(
        &mut self,
        node: &ast::FunctionExpression,
    ) -> types::FunctionType {
        let (param_types, body_type) = self.with_scope(node.span, |s| {
            let mut param_types = Vec::with_capacity(node.params.len());
            for param in node.params.iter() {
                let ty = s.visit_type(&param.type_annotation);
                s.analysis_context.register_symbol(VariableData::pure(
                    param.name.as_str().into(),
                    ty.clone().into(),
                    param.name.span,
                ));
                param_types.push(ty);
            }
            let body_type = s.visit_function_body(&node.body);
            (param_types, body_type)
        });

        self.set_type_at(
            node.span,
            types::FunctionType {
                params: param_types,
                return_type: Box::new(body_type),
            },
        )
    }

    pub fn visit_function_body(&mut self, node: &ast::FunctionBody) -> types::Type {
        let block = match node {
            ast::FunctionBody::Expression(node) => return self.visit_expression(node),
            ast::FunctionBody::TypedBlock(node) => node,
        };

        let ty = if let Some(ref type_annotation) = block.type_annotation {
            self.visit_type(type_annotation)
        } else {
            types::Type::Void
        };
        self.visit_block_expression(&block.block);
        self.check_returns(block, &ty);
        ty
    }

    fn check_returns(&mut self, body: &ast::TypedBlock, expected: &types::Type) {
        let mut returns = Vec::<ast::ReturnStatement>::new();
        body.block.find_returns(&mut returns);

        if returns.len() == 0 && *expected != types::Type::Void {
            self.errors.push(ParseError {
                message: "A function with return annotation needs a return value".into(),
                span: body.block.span,
            });
        }

        for ret in returns {
            let ty = match ret.value {
                Some(value) => self.get_type_at(value.as_span()).unwrap(),
                None => types::Type::Void,
            };
            if !self.can_be_assigned_to(&ty, expected) {
                self.errors.push(ParseError {
                    message: format!("Expected type {}, got {}", expected, ty),
                    span: ret.span,
                })
            }
        }
    }

    fn visit_identifier(&mut self, node: &ast::Identifier) -> types::Type {
        let var = self.analysis_context.lookup_mut(node.as_str());
        let ty = match var {
            Some(handle) => {
                self.analysis_context
                    .add_dependencies(vec![handle.readonly()]);
                handle.add_read();
                (*handle.borrow().ty).clone()
            }
            None => {
                self.errors.push(ParseError {
                    message: format!("Undefined variable: {}", node.as_str()),
                    span: node.span,
                });
                types::Type::Unknown
            }
        };
        self.set_type_at(node.span, ty)
    }

    fn visit_if_expression(&mut self, node: &ast::IfExpression) -> types::Type {
        self.visit_condition(&node.condition);
        let ty = self.with_scope(node.span, |s| s.visit_block_expression(&node.consequent));
        let ty = if let Some(ref alternate) = node.alternate {
            self.visit_alternate(alternate, &ty);
            ty
        } else {
            types::OptionType { some: Box::new(ty) }.into()
        };
        self.set_type_at(node.span, ty)
    }

    fn visit_if_decl_expression(&mut self, node: &ast::IfPatExpression) -> types::Type {
        if !node.pattern.is_refutable() {
            self.error("Refutable pattern expected".into(), node.pattern.as_span());
        };

        let ty = self.with_scope(node.span, |s| {
            let (inferred_type, dependencies) =
                s.with_dependencies(|s| s.visit_expression(&node.scrutinee));
            let mut variables = Vec::<(String, types::Type)>::new();
            s.match_pattern(&node.pattern, inferred_type.clone(), &mut variables);
            for (name, ty) in variables {
                s.analysis_context.register_symbol(VariableData::new(
                    name.clone(),
                    ty.clone().into(),
                    false,
                    node.pattern.as_span(),
                    dependencies.clone(),
                ));
            }
            s.visit_block_expression(&node.consequent)
        });

        let ty = if let Some(ref alternate) = node.alternate {
            self.visit_alternate(alternate, &ty);
            ty
        } else {
            types::OptionType { some: Box::new(ty) }.into()
        };
        self.set_type_at(node.span, ty)
    }

    fn visit_alternate(&mut self, alternate: &ast::Alternate, expected: &types::Type) {
        let alt_ty = match alternate {
            ast::Alternate::Block(b) => self.visit_block_expression(b),
            ast::Alternate::If(i) => self.visit_if_expression(i),
            ast::Alternate::IfDecl(i) => self.visit_if_decl_expression(i),
        };
        if !self.can_be_assigned_to(&alt_ty, expected) {
            self.errors.push(ParseError {
                message: format!(
                    "Branches' types don't match: expected {}, got {}",
                    expected, alt_ty
                ),
                span: alternate.as_span(),
            })
        }
    }

    fn visit_tuple_expression(&mut self, node: &ast::TupleExpression) -> types::TupleType {
        let ty = types::TupleType {
            elements: node
                .elements
                .iter()
                .map(|el| self.visit_expression(el))
                .collect(),
        };
        self.set_type_at(node.span, ty)
    }

    pub fn visit_condition(&mut self, node: &ast::Expression) {
        let condition = self.visit_expression(node);
        if condition != types::Type::Boolean {
            self.errors.push(ParseError {
                message: format!("Condition must evaluate to a boolean, got {}", condition),
                span: node.as_span(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;
    use crate::types::*;

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
        assert_eq!(result, types::Type::Dynamic);
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
        assert_eq!(
            result,
            types::ArrayType {
                element: Box::new(Type::Number)
            }
            .into()
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
        assert_eq!(
            result,
            types::ArrayType {
                element: Box::new(Type::Unknown)
            }
            .into()
        );
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0]
            .message
            .contains("Type mismatch: expected number, found string"));
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
        assert_eq!(result, types::Type::Number);
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
        assert_eq!(
            result,
            types::FunctionType {
                params: vec![Type::Number, types::Type::Number],
                return_type: Box::new(Type::Number),
            }
            .into()
        );
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_identifier() {
        let mut checker = create_type_checker();
        checker.analysis_context.register_symbol(VariableData::pure(
            "x".into(),
            types::Type::Number.into(),
            span("x"),
        ));

        let identifier = ast::Identifier { span: span("x") };

        let result = checker.visit_identifier(&identifier);
        assert_eq!(result, types::Type::Number);
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_expression_or_anonymous() {
        let mut checker = create_type_checker();
        let expression = ast::ExpressionOrAnonymous::Expression(ast::Expression::NumberLiteral(
            ast::NumberLiteral {
                value: ordered_float::OrderedFloat(42.0),
                span: dummy_span(),
            },
        ));

        let result = checker.visit_expression_or_anonymous(&expression);
        assert_eq!(result, types::Type::Number);
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
        assert_eq!(result, types::TupleType { elements: vec![] }.into());
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
        assert_eq!(
            result,
            types::TupleType {
                elements: vec![
                    types::Type::Number,
                    types::Type::String,
                    types::Type::Boolean
                ]
            }
            .into()
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
        assert_eq!(
            result,
            types::TupleType {
                elements: vec![
                    types::Type::Number,
                    types::Type::Tuple(types::TupleType {
                        elements: vec![Type::String, types::Type::Boolean],
                    })
                ]
            }
            .into()
        );
        assert!(checker.errors.is_empty());
    }
}
