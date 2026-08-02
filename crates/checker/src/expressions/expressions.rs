use tine_ast as ast;
use tine_common::diagnostics::DiagnosticKind;
use tine_ir::{self as ir, Typed};
use tine_symbols::symbols::*;
use tine_types::store::TypeStore;
use tine_types::types;

use super::TypeChecker;

impl TypeChecker {
    pub fn visit_expression(&mut self, node: ast::Expression) -> Option<ir::Expression> {
        use ast::Expression::*;
        match node {
            Array(node) => Some(self.visit_array_expression(node).into()),
            Binary(node) => self.visit_binary_expression(node).map(|e| e.into()),
            BooleanLiteral(node) => Some(visit_boolean_literal(node).into()),
            Block(node) => Some(self.visit_block_expression(node).into()),
            Call(node) => self.visit_call_expression(node).map(|n| n.into()),
            ConstructorLiteral(node) => self.visit_constructor_literal(node).map(Into::into),
            Element(node) => self.visit_element_expression(node).map(Into::into),
            FloatLiteral(node) => Some(visit_float_literal(node).into()),
            Member(node) => self.visit_member_expression(node).map(Into::into),
            Function(node) => {
                self.with_scope(|self_| self_.visit_function_expression(node, None).map(Into::into))
            }
            Identifier(node) => self.visit_identifier(node).map(Into::into),
            If(node) => self.visit_if_expression(node).map(Into::into),
            IfDecl(node) => self.visit_if_decl_expression(node).map(Into::into),
            Invalid(_) => None,
            IntLiteral(node) => Some(visit_int_literal(node).into()),
            Intrinsic(node) => Some(self.visit_intrinsic_call(node).into()),
            Loop(node) => self.visit_loop(node),
            Match(node) => self.visit_match_expression(node).map(Into::into),
            StringLiteral(node) => Some(visit_string_literal(node).into()),
            Tuple(node) => self.visit_tuple_expression(node),
            TypeMatch(node) => self.visit_type_match(node).map(Into::into),
            Unary(node) => self.visit_unary_expression(node).map(Into::into),
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
            self.check_assigned_type(
                element_type,
                element.ty(),
                self.is_mutable(element) == Some(false),
                node.loc,
            );
        }

        let ty = self.intern(types::TypeRef {
            inner: TypeStore::ARRAY,
            args: vec![element_type],
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
        let symbol_id = self.get_symbol_id(node.as_str());
        match symbol_id {
            Some(symbol_id) => {
                self.symbols
                    .get_symbol_mut(symbol_id)
                    .access()
                    .read(node.loc);
                Some(ir::Identifier {
                    loc: node.loc,
                    symbol: symbol_id,
                    ty: self.symbol_type_id(symbol_id),
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

    fn visit_intrinsic_call(&mut self, node: ast::IntrinsicCall) -> ir::IntrinsicCall {
        let name = node.name.as_str();
        let (symbol_id, symbol) = self
            .symbols
            .all()
            .filter(|(_, s)| s.defined_at().module() == 0)
            .find(|(_, s)| s.name() == name)
            .expect(&format!("intrinsic call to unknown '{}' function", name));
        let SymbolId::Function(callee) = symbol_id else {
            panic!("expected function symbol")
        };
        let ty = symbol.ty();
        let args = node
            .args
            .into_iter()
            .map(|e| self.visit_expression(e))
            .collect::<Option<Vec<_>>>()
            .expect("got invalid expression(s)");

        ir::IntrinsicCall {
            loc: node.loc,
            callee,
            args,
            ty,
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
                    ..Default::default()
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
        let Some(symbol_id) = self.get_symbol_id(node.constructor.enum_name.name.as_str()) else {
            let error = DiagnosticKind::CannotFindName {
                name: node.constructor.enum_name.name.as_str().to_string(),
            };
            self.error(error, node.constructor.enum_name.loc);
            return None;
        };
        let SymbolId::Enum(e) = symbol_id else {
            self.error(DiagnosticKind::InvalidTypeConstructor, node.constructor.loc);
            return None;
        };
        let e = self.symbols.get(e);
        let variant_name = node.constructor.variant_name?;
        let variant = *e
            .variants
            .iter()
            .find(|&&v| self.symbol_name(v) == variant_name.text)?;
        Some(ir::TypeMatch {
            loc: node.loc,
            expr: Box::new(expression?),
            variant,
        })
    }
}

pub fn visit_boolean_literal(node: ast::BooleanLiteral) -> ir::BooleanLiteral {
    ir::BooleanLiteral {
        loc: node.loc,
        value: node.value,
    }
}

pub fn visit_float_literal(node: ast::FloatLiteral) -> ir::FloatLiteral {
    ir::FloatLiteral {
        loc: node.loc,
        value: *node.value,
    }
}

pub fn visit_int_literal(node: ast::IntLiteral) -> ir::IntLiteral {
    ir::IntLiteral {
        loc: node.loc,
        value: node.value,
    }
}

pub fn visit_string_literal(node: ast::StringLiteral) -> ir::StringLiteral {
    ir::StringLiteral {
        loc: node.loc,
        value: node.text,
    }
}

#[cfg(test)]
mod tests {
    use tine_common::locations::{Location, Span};

    use super::*;

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
    fn test_visit_array_expression_consistent_types() {
        let mut checker = TypeChecker::new();
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
            types::Type::Ref(types::TypeRef {
                inner: TypeStore::ARRAY,
                args: vec![TypeStore::INTEGER],
            })
        );
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn test_visit_array_expression_mixed_types() {
        let mut checker = TypeChecker::new();
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
            types::Type::Ref(types::TypeRef {
                inner: TypeStore::ARRAY,
                args: vec![TypeStore::INTEGER],
            })
        );
        assert_eq!(checker.diagnostics.len(), 1);
    }

    #[test]
    fn test_visit_binary_expression() {
        let mut checker = TypeChecker::new();
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
        let mut checker = TypeChecker::new();
        let id = checker.symbols.insert::<VariableSymbolId>(VariableSymbol {
            name: "x".into(),
            ty: TypeStore::INTEGER,
            defined_at: loc("x"),
            ..Default::default()
        });
        checker.current_scope().bind("x".into(), id.into());

        let identifier = ident("x");

        let result = checker.visit_identifier(identifier);
        let Some(result) = result else {
            panic!("expected Identifier, got {:?}", result)
        };
        assert!(checker.diagnostics.is_empty());
        assert_eq!(result.ty, TypeStore::INTEGER);
    }

    #[test]
    fn test_visit_tuple_expression_empty() {
        let mut checker = TypeChecker::new();
        let tuple_expression = ast::TupleExpression {
            elements: vec![],
            loc: Location::dummy(),
        };

        let result = checker.visit_tuple_expression(tuple_expression);
        let result = checker.resolve(result.unwrap().ty());
        assert_eq!(result, types::Type::Unit);
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn test_visit_tuple_expression_multiple_elements() {
        let mut checker = TypeChecker::new();
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
            types::Type::Tuple(types::TupleType {
                elements: vec![TypeStore::INTEGER, TypeStore::STRING, TypeStore::BOOLEAN],
                ..Default::default()
            })
        );
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn test_visit_tuple_expression_nested() {
        let mut checker = TypeChecker::new();
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
        assert!(matches!(result, types::Type::Tuple(_)));
        assert!(checker.diagnostics.is_empty());
    }
}
