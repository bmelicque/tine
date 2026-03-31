use crate::{
    ast,
    diagnostics::DiagnosticKind,
    ir,
    type_checker::analysis_context::type_store::TypeStore,
    types::{self, ArrayType},
    SymbolKind,
};

use super::TypeChecker;

impl TypeChecker<'_> {
    pub fn visit_expression(&mut self, node: ast::Expression) -> Option<ir::Expression> {
        match node {
            ast::Expression::Array(node) => Some(self.visit_array_expression(node).into()),
            ast::Expression::Binary(node) => self.visit_binary_expression(node).map(|e| e.into()),
            ast::Expression::BooleanLiteral(node) => {
                Some(ir::Expression::BooleanLiteral(ir::BooleanLiteral {
                    loc: node.loc,
                    value: node.value,
                }))
            }
            ast::Expression::Block(node) => Some(self.visit_block_expression(node).into()),
            ast::Expression::Call(node) => self.visit_call_expression(node).map(|n| n.into()),
            ast::Expression::ConstructorLiteral(node) => {
                self.visit_constructor_literal(node).map(Into::into)
            }
            ast::Expression::Element(node) => self.visit_element_expression(node).map(Into::into),
            ast::Expression::FloatLiteral(node) => {
                Some(ir::Expression::FloatLiteral(ir::FloatLiteral {
                    loc: node.loc,
                    value: node.value.into_inner(),
                }))
            }
            ast::Expression::Member(node) => self.visit_member_expression(node).map(Into::into),
            ast::Expression::Function(node) => {
                self.with_scope(|self_| self_.visit_function_expression(node, None).map(Into::into))
            }
            ast::Expression::Identifier(node) => self.visit_identifier(node).map(Into::into),
            ast::Expression::If(node) => self.visit_if_expression(node).map(Into::into),
            ast::Expression::IfDecl(node) => self.visit_if_decl_expression(node).map(Into::into),
            ast::Expression::Invalid(_) => None,
            ast::Expression::IntLiteral(node) => Some(ir::Expression::IntLiteral(ir::IntLiteral {
                loc: node.loc,
                value: node.value,
            })),
            ast::Expression::Loop(node) => self.visit_loop(node),
            ast::Expression::Match(node) => self.visit_match_expression(node).map(Into::into),
            ast::Expression::StringLiteral(node) => {
                Some(ir::Expression::Stringliteral(ir::StringLiteral {
                    loc: node.loc,
                    value: node.text,
                }))
            }
            ast::Expression::Tuple(node) => self.visit_tuple_expression(node),
            ast::Expression::TypeMatch(node) => self.visit_type_match(node).map(Into::into),
            ast::Expression::Unary(node) => self.visit_unary_expression(node).map(Into::into),
        }
    }

    fn visit_array_expression(&mut self, node: ast::ArrayExpression) -> ir::ArrayExpression {
        let elements = node
            .elements
            .into_iter()
            .filter_map(|e| self.visit_expression(e))
            .collect::<Vec<_>>();

        let element_type = elements.first().map_or(TypeStore::DYNAMIC, |e| e.ty());

        for element in &elements {
            self.check_assigned_type(element_type, element.ty(), node.loc);
        }

        let ty = self.intern(ArrayType {
            element: element_type,
        });
        ir::ArrayExpression {
            loc: node.loc,
            elements,
            ty,
        }
    }

    pub fn visit_block_expression(&mut self, node: ast::BlockExpression) -> ir::Block {
        let statements = self.with_scope(|checker| {
            node.statements
                .into_iter()
                .flat_map(|stmt| checker.visit_statement(stmt))
                .collect::<Vec<_>>()
        });

        let ty = statements
            .last()
            .map_or(TypeStore::UNIT, |stmt| match stmt {
                ir::Statement::Expression(e) => e.ty(),
                _ => TypeStore::UNIT,
            });

        ir::Block {
            loc: node.loc,
            statements,
            ty,
        }
    }

    pub fn visit_identifier(&mut self, node: ast::Identifier) -> Option<ir::Identifier> {
        let var = self.lookup_mut(node.as_str());
        match var {
            Some(handle) => {
                handle.read(node.loc);
                self.ctx.add_dependencies(vec![handle.readonly()]);
                Some(ir::Identifier {
                    loc: node.loc,
                    symbol: handle.readonly(),
                })
            }
            None => {
                let error = DiagnosticKind::CannotFindName {
                    name: node.as_str().to_string(),
                };
                self.error(error, node.loc);
                None
            }
        }
    }

    pub fn visit_tuple_expression(&mut self, node: ast::TupleExpression) -> Option<ir::Expression> {
        match node.elements.len() {
            0 => Some(ir::Expression::Tuple(ir::TupleExpression {
                loc: node.loc,
                elements: vec![],
                ty: TypeStore::UNIT,
            })),
            1 => self.visit_expression(node.elements.into_iter().next().unwrap()),
            _ => {
                let elements = node
                    .elements
                    .into_iter()
                    .map(|e| self.visit_expression(e))
                    .collect::<Vec<_>>();
                let elements = if elements.iter().any(|e| e.is_none()) {
                    return None;
                } else {
                    elements.into_iter().filter_map(|e| e).collect::<Vec<_>>()
                };
                let ty = self.intern(types::TupleType {
                    elements: elements.iter().map(|e| e.ty()).collect(),
                });

                Some(ir::Expression::Tuple(ir::TupleExpression {
                    loc: node.loc,
                    elements,
                    ty,
                }))
            }
        }
    }

    fn visit_type_match(&mut self, node: ast::TypeMatch) -> Option<ir::TypeMatch> {
        let expression = node.expression.and_then(|e| self.visit_expression(*e));
        let Some(symbol) = self.lookup(&node.constructor.enum_name.name) else {
            let error = DiagnosticKind::CannotFindName {
                name: node.constructor.enum_name.name,
            };
            self.error(error, node.constructor.enum_name.loc);
            return None;
        };
        let SymbolKind::Enum { variants, .. } = &symbol.borrow().kind else {
            self.error(DiagnosticKind::InvalidTypeConstructor, node.constructor.loc);
            return None;
        };
        let variant_name = node.constructor.variant_name?;
        let variant = variants
            .iter()
            .find(|constructor| constructor.borrow().name == variant_name.text)
            .cloned()?;
        Some(ir::TypeMatch {
            loc: node.loc,
            expr: Box::new(expression?),
            constructor: variant,
        })
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
    use crate::SymbolData;
    use crate::SymbolKind;

    fn create_type_checker() -> TypeChecker<'static> {
        let session = Box::leak(Box::new(Session::new(Box::new(MockLoader))));
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

        let result = checker.visit_array_expression(array_expression);
        assert_eq!(result.ty, TypeStore::DYNAMIC);
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

        let result = checker.visit_array_expression(array_expression);
        let result = checker.resolve(result.ty);
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

        let result = checker.visit_array_expression(array_expression);
        let result = checker.resolve(result.ty);
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
            left: Some(Box::new(ast::Expression::IntLiteral(ast::IntLiteral {
                value: 1,
                loc: Location::dummy(),
            }))),
            right: Some(Box::new(ast::Expression::IntLiteral(ast::IntLiteral {
                value: 2,
                loc: Location::dummy(),
            }))),
            operator: ast::BinaryOperator::Add,
            loc: Location::dummy(),
        };

        let result = checker.visit_binary_expression(binary_expression).unwrap();
        assert_eq!(result.ty, TypeStore::INTEGER);
        assert!(
            checker.diagnostics.is_empty(),
            "expected no errors, got {:?}",
            checker.diagnostics
        );
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

        let Some(result) = checker.visit_identifier(identifier) else {
            panic!()
        };
        assert!(checker.diagnostics.is_empty());
        assert_eq!(result.ty(), TypeStore::INTEGER);
    }

    #[test]
    fn test_visit_tuple_expression_empty() {
        let mut checker = create_type_checker();
        let tuple_expression = ast::TupleExpression {
            elements: vec![],
            loc: Location::dummy(),
        };

        let result = checker.visit_tuple_expression(tuple_expression);
        let result = checker.resolve(result.unwrap().ty());
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

        let result = checker.visit_tuple_expression(tuple_expression);
        let result = checker.resolve(result.unwrap().ty());
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

        let result = checker.visit_tuple_expression(tuple_expression);
        let result = checker.resolve(result.unwrap().ty());
        assert!(matches!(result, Type::Tuple(_)));
        assert!(checker.diagnostics.is_empty());
    }
}
