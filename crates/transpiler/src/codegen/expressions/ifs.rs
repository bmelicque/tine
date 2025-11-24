use crate::codegen::{
    utils::{can_ifexpr_be_inlined, create_ident, undefined, AssignTo},
    CodeGenerator,
};
use mylang_core::ast;
use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

impl CodeGenerator {
    pub fn if_to_swc_expr(&mut self, node: &ast::IfExpression) -> swc::Expr {
        if node.consequent.statements.len() == 0 && node.alternate.is_none() {
            undefined()
        } else if can_ifexpr_be_inlined(node) {
            self.if_to_swc_inlined(node).into()
        } else {
            self.if_to_swc_extracted(node).into()
        }
    }

    fn if_to_swc_inlined(&mut self, node: &ast::IfExpression) -> swc::Expr {
        if let Some(alternate) = &node.alternate {
            let alt = match alternate.as_ref() {
                ast::Alternate::Block(b) => self.block_to_swc_inlined(b).into(),
                ast::Alternate::If(i) => self.if_to_swc_inlined(i).into(),
                ast::Alternate::IfDecl(_) => {
                    panic!("Shouldn't try to inline IfDeclExpression!")
                }
            };
            let alt = Box::new(alt);
            swc::Expr::Cond(swc::CondExpr {
                span: DUMMY_SP,
                test: Box::new(self.expr_to_swc(&node.condition)),
                cons: Box::new(self.block_to_swc_inlined(&node.consequent).into()),
                alt,
            })
        } else {
            let cons = self.block_to_swc_inlined(&node.consequent).into();
            swc::Expr::Cond(swc::CondExpr {
                span: DUMMY_SP,
                test: Box::new(self.expr_to_swc(&node.condition)),
                cons: Box::new(self.some(cons).into()),
                alt: Box::new(self.none().into()),
            })
        }
    }

    fn if_to_swc_extracted(&mut self, node: &ast::IfExpression) -> swc::Expr {
        let is_option = node.alternate.is_none();
        let id = self.add_temp_var_to_current_block();
        self.enter_block();
        let if_stmt = self.if_to_swc_stmt(node, AssignTo::Last(id.clone()));
        self.exit_block();
        self.push_to_block(if_stmt.into());
        if is_option {
            let stmt = self.into_option(&id);
            self.push_to_block(stmt);
        }
        create_ident(&id).into()
    }

    pub fn if_decl_to_swc_expr(&mut self, node: &ast::IfPatExpression) -> swc::Expr {
        let is_option = node.alternate.is_none();
        let id = self.add_temp_var_to_current_block();
        self.enter_block();
        let if_stmt = self.if_decl_to_swc_stmt(node, AssignTo::Last(id.clone()));
        self.exit_block();
        self.push_to_block(if_stmt.into());
        if is_option {
            let stmt = self.into_option(&id);
            self.push_to_block(stmt);
        }
        create_ident(&id).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mylang_core::{ast, CheckedModule};
    use swc_ecma_ast as swc;

    fn dummy_span() -> pest::Span<'static> {
        pest::Span::new("", 0, 0).unwrap()
    }

    fn mock_expr() -> ast::Expression {
        ast::Expression::NumberLiteral(ast::NumberLiteral {
            value: 1.0.into(),
            span: dummy_span(),
        })
    }

    fn mock_block() -> ast::BlockExpression {
        ast::BlockExpression {
            statements: vec![ast::Statement::Expression(ast::ExpressionStatement {
                expression: mock_expr().into(),
            })],
            span: dummy_span(),
        }
    }

    fn mock_if_expr(with_alt: bool) -> ast::IfExpression {
        ast::IfExpression {
            condition: mock_expr().into(),
            consequent: Box::new(mock_block()),
            alternate: if with_alt {
                Some(Box::new(ast::Alternate::Block(mock_block())))
            } else {
                None
            },
            span: dummy_span(),
        }
    }

    impl CodeGenerator {
        fn new_for_test() -> Self {
            let mut gen = CodeGenerator::new(
                swc_common::FileName::Custom("".into()),
                CheckedModule::dummy(),
            );
            gen.enter_block();
            gen
        }
    }

    #[test]
    fn returns_undefined_for_empty_if() {
        let mut gen = CodeGenerator::new_for_test();
        let node = ast::IfExpression {
            condition: mock_expr().into(),
            consequent: Box::new(ast::BlockExpression {
                statements: vec![],
                span: dummy_span(),
            }),
            alternate: None,
            span: dummy_span(),
        };

        let result = gen.if_to_swc_expr(&node);
        match result {
            swc::Expr::Ident(ident) => {
                assert_eq!(ident.sym.to_string(), "undefined");
            }
            _ => panic!("Expected undefined identifier, got {:?}", result),
        }
    }

    #[test]
    fn generates_cond_expr_for_inlined_if() {
        let mut gen = CodeGenerator::new_for_test();
        let node = mock_if_expr(true);

        assert!(crate::codegen::utils::can_ifexpr_be_inlined(&node));

        let result = gen.if_to_swc_expr(&node);
        match result {
            swc::Expr::Cond(cond) => {
                assert!(matches!(*cond.test, swc::Expr::Lit(_)));
                assert!(matches!(*cond.cons, swc::Expr::Lit(_)));
                assert!(matches!(*cond.alt, swc::Expr::Call(_) | swc::Expr::Lit(_)));
            }
            _ => panic!("Expected CondExpr, got {:?}", result),
        }
    }

    #[test]
    fn generates_cond_expr_for_inlined_if_with_else() {
        let mut gen = CodeGenerator::new_for_test();
        let node = mock_if_expr(true);

        let result = gen.if_to_swc_inlined(&node);
        match result {
            swc::Expr::Cond(cond) => {
                assert!(matches!(*cond.alt, swc::Expr::Cond(_) | swc::Expr::Call(_)));
            }
            _ => panic!("Expected CondExpr, got {:?}", result),
        }
    }

    #[test]
    fn generates_temp_var_for_extracted_if() {
        let mut gen = CodeGenerator::new_for_test();
        let node = mock_if_expr(false);

        let result = gen.if_to_swc_extracted(&node);
        match result {
            swc::Expr::Ident(ident) => {
                assert!(ident.sym.to_string().starts_with("__"));
            }
            _ => panic!("Expected identifier for temp var, got {:?}", result),
        }
    }
}
