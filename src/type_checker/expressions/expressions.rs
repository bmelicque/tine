use std::collections::HashMap;

use crate::{
    ast,
    parser::parser::ParseError,
    types::{StructField, Type},
};

use super::TypeChecker;
use crate::type_checker::VariableInfo;

impl TypeChecker {
    pub fn visit_expression(&mut self, node: &ast::Expression) -> Type {
        match node {
            ast::Expression::Array(node) => self.visit_array_expression(node),
            ast::Expression::Binary(node) => self.visit_binary_expression(node),
            ast::Expression::BooleanLiteral(_) => Type::Boolean,
            ast::Expression::Block(node) => self.visit_block_expression(node),
            ast::Expression::CompositeLiteral(node) => self.visit_composite_literal(node),
            ast::Expression::Empty => Type::Void,
            ast::Expression::FieldAccess(node) => self.visit_field_access_expression(node),
            ast::Expression::Function(node) => self.visit_function_expression(node),
            ast::Expression::Identifier(node) => self.visit_identifier(node),
            ast::Expression::If(node) => self.visit_if_expression(node),
            ast::Expression::IfDecl(node) => self.visit_if_decl_expression(node),
            ast::Expression::Loop(node) => self.visit_loop(node),
            ast::Expression::Match(node) => self.visit_match_expression(node),
            ast::Expression::NumberLiteral(_) => Type::Number,
            ast::Expression::StringLiteral(_) => Type::String,
            ast::Expression::Tuple(node) => self.visit_tuple_expression(node),
            ast::Expression::TupleIndexing(node) => self.visit_tuple_indexing(node),
        }
    }

    pub fn visit_expression_or_anonymous(&mut self, node: &ast::ExpressionOrAnonymous) -> Type {
        match node {
            ast::ExpressionOrAnonymous::Expression(node) => self.visit_expression(node),
            ast::ExpressionOrAnonymous::Struct(node) => self.visit_anonymous_struct_literal(node),
        }
    }

    fn visit_array_expression(&mut self, node: &ast::ArrayExpression) -> Type {
        if node.elements.len() == 0 {
            return Type::Dynamic;
        }

        let mut ty = Type::Dynamic;
        for value in node.elements.iter() {
            let value_ty = self.visit_expression(value);
            if ty == Type::Dynamic {
                ty = value_ty;
                continue;
            }
            if !value_ty.is_assignable_to(&ty) {
                self.errors.push(ParseError {
                    message: format!("Type mismatch: expected {}, found {}", ty, value_ty),
                    span: value.as_span(),
                });
                ty = Type::Unknown;
            }
        }

        Type::Array(Box::new(ty))
    }

    fn visit_binary_expression(&mut self, node: &ast::BinaryExpression) -> Type {
        let left_type = self.visit_expression(&node.left);
        let right_type = self.visit_expression(&node.right);

        let mut push_error = |ty: Type| {
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
                if !matches!(left_type, Type::Unknown | Type::Number) {
                    push_error(left_type);
                };
                if !matches!(right_type, Type::Unknown | Type::Number) {
                    push_error(right_type);
                };
            }
            ast::BinaryOperator::Eq | ast::BinaryOperator::Neq => {
                if left_type != right_type
                    && left_type != Type::Unknown
                    && right_type != Type::Unknown
                {
                    self.errors.push(ParseError {
                        message: format!(
                            "Types {:?} and {:?} cannot be compared",
                            left_type, right_type
                        ),
                        span: node.span,
                    });
                    return Type::Unknown;
                }
            }
            ast::BinaryOperator::LAnd | ast::BinaryOperator::LOr => {
                if !matches!(left_type, Type::Unknown | Type::Boolean) {
                    push_error(left_type);
                };
                if !matches!(right_type, Type::Unknown | Type::Boolean) {
                    push_error(right_type);
                };
            }
        };

        match node.operator {
            ast::BinaryOperator::Add
            | ast::BinaryOperator::Sub
            | ast::BinaryOperator::Mul
            | ast::BinaryOperator::Div
            | ast::BinaryOperator::Mod
            | ast::BinaryOperator::Pow => Type::Number,
            ast::BinaryOperator::Eq
            | ast::BinaryOperator::Geq
            | ast::BinaryOperator::Grt
            | ast::BinaryOperator::LAnd
            | ast::BinaryOperator::Leq
            | ast::BinaryOperator::Less
            | ast::BinaryOperator::LOr
            | ast::BinaryOperator::Neq => Type::Boolean,
        }
    }

    pub fn visit_block_expression(&mut self, node: &ast::BlockExpression) -> Type {
        // TODO: handle diverging statements (return, break, continue)
        let mut ty = Type::Void;
        for stmt in node.statements.iter() {
            ty = self.visit_statement(&stmt);
        }
        ty
    }

    fn visit_field_access_expression(&mut self, node: &ast::FieldAccessExpression) -> Type {
        let mut ty = self.visit_expression(&node.object);
        let type_str = format!("{}", ty.clone());
        while let Type::Named { ref name, args } = ty {
            let mut substitutions = HashMap::new();
            let params = self.type_registry.get_type_params(name);
            for (i, param) in params.iter().enumerate() {
                let substitute = match args.get(i) {
                    Some(arg) => arg.clone(),
                    None => Type::Dynamic,
                };
                substitutions.insert(param, substitute);
            }
            let raw = self.type_registry.lookup(name).unwrap();
            ty = substitute_type(&raw, &substitutions);
        }

        let prop = node.prop.as_str();
        match ty {
            Type::Struct { fields } => match fields.iter().find(|field| field.name == prop) {
                Some(field) => field.def.clone(),
                None => {
                    self.errors.push(ParseError {
                        message: format!(
                            "Property '{}' does not exist on type '{}'",
                            prop, type_str
                        ),
                        span: node.span,
                    });
                    Type::Unknown
                }
            },
            _ => {
                self.errors.push(ParseError {
                    message: format!("Property '{}' does not exist on type '{}'", prop, type_str),
                    span: node.span,
                });
                Type::Unknown
            }
        }
    }

    fn visit_function_expression(&mut self, node: &ast::FunctionExpression) -> Type {
        self.symbols.enter_scope();

        let mut param_types = Vec::new();
        for param in node.params.iter() {
            let ty = self.resolve_type(&param.type_annotation);
            self.symbols.define(param.name.as_str(), ty.clone(), false);
            param_types.push(ty);
        }
        let body_type = self.visit_function_body(&node.body);

        self.symbols.exit_scope();

        Type::Function {
            params: param_types,
            return_type: Box::new(body_type),
        }
    }

    fn visit_function_body(&mut self, node: &ast::FunctionBody) -> Type {
        let block = match node {
            ast::FunctionBody::Expression(node) => return self.visit_expression(node),
            ast::FunctionBody::TypedBlock(node) => node,
        };

        let ty = if let Some(ref type_annotation) = block.type_annotation {
            self.visit_type(type_annotation)
        } else {
            Type::Void
        };
        self.visit_block_expression(&block.block);
        self.check_returns(block, &ty);
        ty
    }

    // FIXME: this will re-visit the nodes!
    fn check_returns(&mut self, body: &ast::TypedBlock, expected: &Type) {
        let mut returns = Vec::<ast::ReturnStatement>::new();
        body.block.find_returns(&mut returns);

        if returns.len() == 0 && *expected != Type::Void {
            self.errors.push(ParseError {
                message: "A function with return annotation needs a return value".into(),
                span: body.block.span,
            });
        }

        for ret in returns {
            let ty = match ret.value {
                Some(value) => self.visit_expression(&value),
                None => Type::Void,
            };
            if !ty.is_assignable_to(expected) {
                self.errors.push(ParseError {
                    message: format!("Expected type {}, got {}", expected, ty),
                    span: ret.span,
                })
            }
        }
    }

    fn visit_identifier(&mut self, node: &ast::Identifier) -> Type {
        match self.symbols.lookup(node.as_str()) {
            Some(VariableInfo { ty, .. }) => ty.clone(),
            None => {
                self.errors.push(ParseError {
                    message: format!("Undefined variable: {}", node.as_str()),
                    span: node.span,
                });
                Type::Unknown
            }
        }
    }

    fn visit_if_expression(&mut self, node: &ast::IfExpression) -> Type {
        self.symbols.enter_scope();
        self.visit_condition(&node.condition);
        let ty = self.visit_block_expression(&node.consequent);
        self.symbols.exit_scope();
        if let Some(ref alternate) = node.alternate {
            self.visit_alternate(alternate, &ty);
            ty
        } else {
            Type::Option(Box::new(ty))
        }
    }

    fn visit_if_decl_expression(&mut self, node: &ast::IfDeclExpression) -> Type {
        if !node.pattern.is_refutable() {
            self.errors.push(ParseError {
                message: "Refutable pattern expected".into(),
                span: node.pattern.as_span(),
            });
        };
        self.symbols.enter_scope();
        let inferred_type = self.visit_expression(&node.scrutinee);
        let mut variables = Vec::<(String, Type)>::new();
        self.match_pattern(&node.pattern, inferred_type, &mut variables);
        for (name, ty) in variables {
            self.symbols.define(&name, ty, false);
        }
        let ty = self.visit_block_expression(&node.consequent);
        self.symbols.exit_scope();
        if let Some(ref alternate) = node.alternate {
            self.visit_alternate(alternate, &ty);
            ty
        } else {
            Type::Option(Box::new(ty))
        }
    }

    fn visit_alternate(&mut self, alternate: &ast::Alternate, expected: &Type) {
        let alt_ty = match alternate {
            ast::Alternate::Block(b) => self.visit_block_expression(b),
            ast::Alternate::If(i) => self.visit_if_expression(i),
            ast::Alternate::IfDecl(i) => self.visit_if_decl_expression(i),
        };
        if !alt_ty.is_assignable_to(expected) {
            self.errors.push(ParseError {
                message: format!(
                    "Branches' types don't match: expected {}, got {}",
                    expected, alt_ty
                ),
                span: alternate.as_span(),
            })
        }
    }

    fn visit_tuple_expression(&mut self, node: &ast::TupleExpression) -> Type {
        Type::Tuple(
            node.elements
                .iter()
                .map(|el| self.visit_expression(el))
                .collect(),
        )
    }

    pub fn visit_tuple_indexing(&mut self, node: &ast::TupleIndexingExpression) -> Type {
        let left_type = self.visit_expression(&node.tuple);
        let Type::Tuple(tuple) = self.unwrap_named_type(&left_type) else {
            self.errors.push(ParseError {
                message: format!("Expected tuple type, got {}", left_type),
                span: node.tuple.as_span(),
            });
            return Type::Unknown;
        };
        let value = node.index.value;
        if value != value.round() {
            self.errors.push(ParseError {
                message: "Integer expected".into(),
                span: node.index.span,
            });
            return Type::Unknown;
        }
        let value = value as isize;
        if value < 0 {
            self.errors.push(ParseError {
                message: "Index out of range".into(),
                span: node.index.span,
            });
            return Type::Unknown;
        }
        let value = value as usize;
        if value >= tuple.len() {
            self.errors.push(ParseError {
                message: "Index out of range".into(),
                span: node.index.span,
            });
            Type::Unknown
        } else {
            tuple[value].clone()
        }
    }

    pub fn visit_condition(&mut self, node: &ast::Expression) {
        let condition = self.visit_expression(node);
        if condition != Type::Boolean {
            self.errors.push(ParseError {
                message: format!("Condition must evaluate to a boolean, got {}", condition),
                span: node.as_span(),
            });
        }
    }
}

fn substitute_type(ty: &Type, substitutions: &HashMap<&String, Type>) -> Type {
    match ty {
        // If the type is a named type, substitute it if it's in the map
        Type::Named { name, args: _ } => {
            if let Some(substituted) = substitutions.get(name) {
                substituted.clone()
            } else {
                // FIXME: substitute args
                // for arg in args {
                // }
                ty.clone()
            }
        }

        // Handle arrays recursively
        Type::Array(inner) => Type::Array(Box::new(substitute_type(inner, substitutions))),

        // Handle options recursively
        Type::Option(inner) => Type::Option(Box::new(substitute_type(inner, substitutions))),

        // Handle maps recursively
        Type::Map { key, value } => Type::Map {
            key: Box::new(substitute_type(key, substitutions)),
            value: Box::new(substitute_type(value, substitutions)),
        },

        // Handle structs recursively
        Type::Struct { fields } => Type::Struct {
            fields: fields
                .iter()
                .map(|field| StructField {
                    name: field.name.clone(),
                    def: substitute_type(&field.def, substitutions),
                    optional: field.optional,
                })
                .collect(),
        },

        // Handle other types (e.g., primitives) as is
        _ => ty.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;
    use crate::types::{StructField, Type};

    fn create_type_checker() -> TypeChecker {
        TypeChecker::new()
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
        assert_eq!(result, Type::Dynamic);
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_array_expression_consistent_types() {
        let mut checker = create_type_checker();
        let array_expression = ast::ArrayExpression {
            elements: vec![
                ast::Expression::NumberLiteral(ast::NumberLiteral {
                    value: 1.0,
                    span: dummy_span(),
                }),
                ast::Expression::NumberLiteral(ast::NumberLiteral {
                    value: 2.0,
                    span: dummy_span(),
                }),
                ast::Expression::NumberLiteral(ast::NumberLiteral {
                    value: 3.0,
                    span: dummy_span(),
                }),
            ],
            span: dummy_span(),
        };

        let result = checker.visit_array_expression(&array_expression);
        assert_eq!(result, Type::Array(Box::new(Type::Number)));
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_array_expression_mixed_types() {
        let mut checker = create_type_checker();
        let array_expression = ast::ArrayExpression {
            elements: vec![
                ast::Expression::NumberLiteral(ast::NumberLiteral {
                    value: 1.0,
                    span: dummy_span(),
                }),
                ast::Expression::StringLiteral(ast::StringLiteral {
                    span: span("hello"),
                }),
            ],
            span: dummy_span(),
        };

        let result = checker.visit_array_expression(&array_expression);
        assert_eq!(result, Type::Array(Box::new(Type::Unknown)));
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
                value: 1.0,
                span: dummy_span(),
            })),
            right: Box::new(ast::Expression::NumberLiteral(ast::NumberLiteral {
                value: 2.0,
                span: dummy_span(),
            })),
            operator: ast::BinaryOperator::Add,
            span: dummy_span(),
        };

        let result = checker.visit_binary_expression(&binary_expression);
        assert_eq!(result, Type::Number);
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_field_access_expression() {
        let mut checker = create_type_checker();
        checker.type_registry.define(
            "User",
            Type::Struct {
                fields: vec![
                    StructField {
                        name: "name".to_string(),
                        def: Type::String,
                        optional: false,
                    },
                    StructField {
                        name: "age".to_string(),
                        def: Type::Number,
                        optional: false,
                    },
                ],
            },
            None,
        );

        let field_access_expression = ast::FieldAccessExpression {
            object: Box::new(ast::Expression::Identifier(ast::Identifier {
                span: span("user"),
            })),
            prop: ast::Identifier { span: span("name") },
            span: dummy_span(),
        };

        checker.symbols.define(
            "user",
            Type::Named {
                name: "User".to_string(),
                args: vec![],
            },
            false,
        );

        let result = checker.visit_field_access_expression(&field_access_expression);
        assert!(
            checker.errors.is_empty(),
            "Expected no errors, got {:?}",
            checker.errors
        );
        assert_eq!(result, Type::String);
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
            Type::Function {
                params: vec![Type::Number, Type::Number],
                return_type: Box::new(Type::Number),
            }
        );
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_identifier() {
        let mut checker = create_type_checker();
        checker.symbols.define("x", Type::Number, false);

        let identifier = ast::Identifier { span: span("x") };

        let result = checker.visit_identifier(&identifier);
        assert_eq!(result, Type::Number);
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_expression_or_anonymous() {
        let mut checker = create_type_checker();
        let expression = ast::ExpressionOrAnonymous::Expression(ast::Expression::NumberLiteral(
            ast::NumberLiteral {
                value: 42.0,
                span: dummy_span(),
            },
        ));

        let result = checker.visit_expression_or_anonymous(&expression);
        assert_eq!(result, Type::Number);
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
        assert_eq!(result, Type::Tuple(vec![]));
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_tuple_expression_multiple_elements() {
        let mut checker = create_type_checker();
        let tuple_expression = ast::TupleExpression {
            elements: vec![
                ast::Expression::NumberLiteral(ast::NumberLiteral {
                    value: 42.0,
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
            Type::Tuple(vec![Type::Number, Type::String, Type::Boolean])
        );
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_tuple_expression_nested() {
        let mut checker = create_type_checker();
        let tuple_expression = ast::TupleExpression {
            elements: vec![
                ast::Expression::NumberLiteral(ast::NumberLiteral {
                    value: 1.0,
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
            Type::Tuple(vec![
                Type::Number,
                Type::Tuple(vec![Type::String, Type::Boolean])
            ])
        );
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_tuple_indexing_valid() {
        let mut checker = create_type_checker();
        let tuple_type = Type::Tuple(vec![Type::Number, Type::String, Type::Boolean]);

        checker
            .symbols
            .define("my_tuple", tuple_type.clone(), false);

        let tuple_indexing = ast::TupleIndexingExpression {
            tuple: Box::new(ast::Expression::Identifier(ast::Identifier {
                span: span("my_tuple"),
            })),
            index: ast::NumberLiteral {
                value: 1.0,
                span: dummy_span(),
            },
            span: dummy_span(),
        };

        let result = checker.visit_tuple_indexing(&tuple_indexing);
        assert_eq!(result, Type::String);
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_tuple_indexing_invalid_type() {
        let mut checker = create_type_checker();
        checker.symbols.define("not_a_tuple", Type::Number, false);

        let tuple_indexing = ast::TupleIndexingExpression {
            tuple: Box::new(ast::Expression::Identifier(ast::Identifier {
                span: span("not_a_tuple"),
            })),
            index: ast::NumberLiteral {
                value: 0.0,
                span: dummy_span(),
            },
            span: dummy_span(),
        };

        let result = checker.visit_tuple_indexing(&tuple_indexing);
        assert_eq!(result, Type::Unknown);
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0]
            .message
            .contains("Expected tuple type, got number"));
    }

    #[test]
    fn test_visit_tuple_indexing_non_integer_index() {
        let mut checker = create_type_checker();
        let tuple_type = Type::Tuple(vec![Type::Number, Type::String]);

        checker.symbols.define("my_tuple", tuple_type, false);

        let tuple_indexing = ast::TupleIndexingExpression {
            tuple: Box::new(ast::Expression::Identifier(ast::Identifier {
                span: span("my_tuple"),
            })),
            index: ast::NumberLiteral {
                value: 1.5,
                span: dummy_span(),
            },
            span: dummy_span(),
        };

        let result = checker.visit_tuple_indexing(&tuple_indexing);
        assert_eq!(result, Type::Unknown);
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].message.contains("Integer expected"));
    }

    #[test]
    fn test_visit_tuple_indexing_out_of_range() {
        let mut checker = create_type_checker();
        let tuple_type = Type::Tuple(vec![Type::Number, Type::String]);

        checker.symbols.define("my_tuple", tuple_type, false);

        let tuple_indexing = ast::TupleIndexingExpression {
            tuple: Box::new(ast::Expression::Identifier(ast::Identifier {
                span: span("my_tuple"),
            })),
            index: ast::NumberLiteral {
                value: 2.0,
                span: dummy_span(),
            },
            span: dummy_span(),
        };

        let result = checker.visit_tuple_indexing(&tuple_indexing);
        assert_eq!(result, Type::Unknown);
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].message.contains("Index out of range"));
    }

    #[test]
    fn test_visit_tuple_indexing_negative_index() {
        let mut checker = create_type_checker();
        let tuple_type = Type::Tuple(vec![Type::Number, Type::String]);

        checker.symbols.define("my_tuple", tuple_type, false);

        let tuple_indexing = ast::TupleIndexingExpression {
            tuple: Box::new(ast::Expression::Identifier(ast::Identifier {
                span: span("my_tuple"),
            })),
            index: ast::NumberLiteral {
                value: -1.0,
                span: dummy_span(),
            },
            span: dummy_span(),
        };

        let result = checker.visit_tuple_indexing(&tuple_indexing);
        assert_eq!(result, Type::Unknown);
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].message.contains("Index out of range"));
    }
}
