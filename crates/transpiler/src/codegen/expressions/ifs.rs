use crate::codegen::{
    expressions::utils::stmt_to_iife,
    utils::{can_ifexpr_be_inlined, undefined},
    CodeGenerator,
};
use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use tine_core::ir;

impl CodeGenerator<'_> {
    pub fn if_to_swc_expr(&mut self, node: &ir::IfExpression) -> swc::Expr {
        if node.consequent.statements.len() == 0 && node.alternate.is_none() {
            undefined()
        } else if can_ifexpr_be_inlined(node) {
            self.if_to_swc_inlined(node).into()
        } else {
            self.if_to_swc_iife(node)
        }
    }

    fn if_to_swc_inlined(&mut self, node: &ir::IfExpression) -> swc::CondExpr {
        let test = Box::new(self.expr_to_swc(&node.condition));
        let cons = Box::new(self.block_to_swc_inlined(&node.consequent).into());
        let alt = Box::new(node.alternate.as_ref().map_or(self.none().into(), |alt| {
            self.block_to_swc_inlined(alt).into()
        }));

        swc::CondExpr {
            span: DUMMY_SP,
            test,
            cons,
            alt,
        }
    }

    fn if_to_swc_iife(&mut self, node: &ir::IfExpression) -> swc::Expr {
        stmt_to_iife(self.if_to_swc_stmt(node).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use swc_ecma_ast as swc;
    use tine_core::{Location, ModuleLoader, Session, TypeStore};

    fn mock_expr() -> ir::Expression {
        ir::Expression::IntLiteral(ir::IntLiteral {
            value: 1,
            loc: Location::dummy(),
        })
    }

    fn mock_block() -> ir::Block {
        ir::Block {
            statements: vec![ir::Statement::Expression(mock_expr())],
            loc: Location::dummy(),
            ty: TypeStore::INTEGER,
        }
    }

    fn mock_if_expr(with_alt: bool) -> ir::IfExpression {
        ir::IfExpression {
            loc: Location::dummy(),
            condition: Box::new(mock_expr()),
            consequent: mock_block(),
            alternate: if with_alt { Some(mock_block()) } else { None },
            ty: TypeStore::UNKNOWN,
        }
    }

    struct MockLoader;
    impl ModuleLoader for MockLoader {
        fn load(&self, _: &tine_core::ModulePath) -> anyhow::Result<String> {
            Ok("".to_string())
        }
    }

    impl CodeGenerator<'_> {
        fn new_for_test() -> Self {
            let session = Box::leak(Box::new(Session::new(Box::new(MockLoader))));
            CodeGenerator::new(session, 0)
        }
    }

    #[test]
    fn returns_undefined_for_empty_if() {
        let mut gen = CodeGenerator::new_for_test();
        let node = ir::IfExpression {
            loc: Location::dummy(),
            condition: Box::new(mock_expr()),
            consequent: ir::Block {
                loc: Location::dummy(),
                statements: vec![],
                ty: TypeStore::UNIT,
            },
            alternate: None,
            ty: TypeStore::UNKNOWN,
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
        assert!(
            matches!(*result.alt, swc::Expr::Lit(_)),
            "got {:?}",
            result.alt
        );
    }
}
