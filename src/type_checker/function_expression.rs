use crate::{ast::Node, parser::parser::ParseError, types::Type};

use super::type_checker::TypeChecker;

impl TypeChecker {
    pub(super) fn visit_function_expression(&mut self, node: &Node) -> Type {
        let Node::FunctionExpression {
            parameters,
            return_type,
            body,
        } = node
        else {
            panic!("This should be a function expression!")
        };

        let mut param_types = Vec::new();

        self.symbols.enter_scope();

        if let Some(params) = parameters {
            for param in params {
                let ty = if let Some(ann) = &param.type_annotation {
                    self.resolve_type(ann)
                } else {
                    Type::Unknown // type inference could go here later
                };
                self.symbols.define(&param.name, ty.clone(), true);
                param_types.push(ty);
            }
        }

        let body_type = if let Some(body) = body {
            self.visit(body)
        } else {
            Type::Unknown
        };

        self.symbols.exit_scope();

        let return_ty = if let Some(return_annotation) = return_type {
            let expected = self.resolve_type(return_annotation);
            if body_type != Type::Unknown && expected != body_type {
                self.errors.push(ParseError {
                    message: format!(
                        "Return type mismatch: expected {:?}, got {:?}",
                        expected, body_type,
                    ),
                    span: body.clone().unwrap().span.clone(),
                });
            }
            expected
        } else {
            body_type
        };

        Type::Function {
            params: param_types,
            return_type: Box::new(return_ty),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::{Node, Parameter, Spanned},
        type_checker::TypeChecker,
        types::Type,
    };

    fn dummy_span() -> pest::Span<'static> {
        pest::Span::new("", 0, 0).unwrap()
    }

    fn spanned<T>(node: T) -> Spanned<T> {
        Spanned {
            node,
            span: dummy_span(),
        }
    }

    #[test]
    fn infers_type_for_unannotated_function_expression() {
        let expr_node = Node::FunctionExpression {
            parameters: Some(vec![Parameter {
                name: "x".into(),
                type_annotation: None,
            }]),
            return_type: None,
            body: Some(Box::new(spanned(Node::Identifier("x".into())))),
        };

        let mut checker = TypeChecker::new();
        let ty = checker.visit_function_expression(&expr_node);

        match ty {
            Type::Function {
                params,
                return_type,
            } => {
                assert_eq!(params.len(), 1);
                assert_eq!(params[0], Type::Unknown);
                assert_eq!(*return_type, Type::Unknown);
            }
            _ => panic!("Expected function type"),
        }
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn resolves_type_annotations() {
        let expr_node = Node::FunctionExpression {
            parameters: Some(vec![Parameter {
                name: "x".into(),
                type_annotation: Some("number".into()),
            }]),
            return_type: Some("boolean".into()),
            body: Some(Box::new(spanned(Node::BooleanLiteral(true)))),
        };

        let mut checker = TypeChecker::new();
        let ty = checker.visit_function_expression(&expr_node);

        match ty {
            Type::Function {
                params,
                return_type,
            } => {
                assert_eq!(params, vec![Type::Number]);
                assert_eq!(*return_type, Type::Boolean);
            }
            _ => panic!("Expected function type"),
        }

        assert!(checker.errors.is_empty());
    }

    #[test]
    fn detects_return_type_mismatch() {
        let expr_node = Node::FunctionExpression {
            parameters: Some(vec![Parameter {
                name: "x".into(),
                type_annotation: Some("number".into()),
            }]),
            return_type: Some("boolean".into()),
            body: Some(Box::new(spanned(Node::NumberLiteral(42.0)))),
        };

        let mut checker = TypeChecker::new();
        let ty = checker.visit_function_expression(&expr_node);

        assert_eq!(
            ty,
            Type::Function {
                params: vec![Type::Number],
                return_type: Box::new(Type::Boolean),
            }
        );

        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].message.contains("Return type mismatch"));
    }
}
