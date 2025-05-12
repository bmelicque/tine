use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

use crate::ast;

use super::{utils::create_ident, CodeGenerator};

impl CodeGenerator {
    pub fn stmt_to_swc(&mut self, node: ast::Statement) -> Option<swc::Stmt> {
        match node {
            ast::Statement::Assignment(node) => Some(self.assignment_to_swc(node).into()),
            ast::Statement::Block(node) => Some(self.block_to_swc(node).into()),
            ast::Statement::Empty => None,
            ast::Statement::Expression(node) => Some(
                swc::ExprStmt {
                    span: DUMMY_SP,
                    expr: Box::new(self.expr_to_swc(*node.expression)),
                }
                .into(),
            ),
            ast::Statement::Return(node) => Some(self.return_to_swc(node).into()),
            ast::Statement::TypeAlias(node) => self.alias_to_swc(node).into(),
            ast::Statement::VariableDeclaration(node) => Some(self.declaration_to_swc(node).into()),
        }
    }

    fn assignment_to_swc(&mut self, node: ast::Assignment) -> swc::ExprStmt {
        swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(swc::Expr::Assign(swc::AssignExpr {
                span: DUMMY_SP,
                op: swc::AssignOp::Assign,
                left: swc::PatOrExpr::Expr(Box::new(create_ident(&node.name).into())),
                right: Box::new(self.expr_to_swc(node.value)),
            })),
        }
    }

    fn block_to_swc(&mut self, node: ast::BlockStatement) -> swc::BlockStmt {
        self.push_scope();
        let stmts: Vec<swc::Stmt> = node
            .statements
            .iter()
            .filter_map(|stmt| self.stmt_to_swc(stmt.clone()))
            .collect();
        self.drop_scope();

        swc::BlockStmt {
            span: DUMMY_SP,
            stmts,
        }
    }

    fn declaration_to_swc(&mut self, node: ast::VariableDeclaration) -> swc::Decl {
        let init = Some(Box::new(self.expr_to_swc(*node.value)));

        let decl = swc::VarDeclarator {
            span: DUMMY_SP,
            name: self.pattern_to_swc(*node.pattern),
            init,
            definite: false,
        };

        let kind = match node.op {
            ast::DeclarationOp::Mut => swc::VarDeclKind::Let,
            ast::DeclarationOp::Const => swc::VarDeclKind::Const,
        };

        swc::Decl::Var(Box::new(swc::VarDecl {
            span: DUMMY_SP,
            kind,
            declare: false,
            decls: vec![decl],
        }))
    }

    fn return_to_swc(&mut self, node: ast::ReturnStatement) -> swc::ReturnStmt {
        swc::ReturnStmt {
            span: DUMMY_SP,
            arg: node
                .value
                .map(|value| self.expr_to_swc(*value))
                .map(Box::new),
        }
    }
}
