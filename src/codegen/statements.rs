use core::panic;
use std::error::Error;
use swc_common::DUMMY_SP;
use swc_ecma_ast as ast;

use crate::ast::Node;

use super::{utils::create_ident, CodeGenerator};

impl CodeGenerator {
    pub fn node_to_swc_stmt(&mut self, node: Node) -> Result<Option<ast::Stmt>, Box<dyn Error>> {
        match node {
            Node::VariableDeclaration { .. } => self.declaration_to_swc_statement(node),
            Node::TypeDeclaration { .. } => self.type_declaration_to_swc_decl(node),
            Node::Assignment { name, value } => {
                let value_expr = if let Some(v) = value {
                    self.node_to_swc_expr(v.node)
                } else {
                    panic!("Missing expression in assignment!");
                };

                let Some(name) = name else {
                    panic!("Missing variable name in assignment.");
                };

                let swc_name = create_ident(&name);

                let swc_assignee = ast::Expr::Ident(swc_name);

                Ok(Some(ast::Stmt::Expr(ast::ExprStmt {
                    span: DUMMY_SP,
                    expr: Box::new(ast::Expr::Assign(ast::AssignExpr {
                        span: DUMMY_SP,
                        op: ast::AssignOp::Assign,
                        left: ast::PatOrExpr::Expr(Box::new(swc_assignee)),
                        right: Box::new(value_expr),
                    })),
                })))
            }
            Node::ReturnStatement(expr) => {
                let arg = if let Some(e) = expr {
                    let swc_expr = self.node_to_swc_expr(e.node);
                    Some(Box::new(swc_expr))
                } else {
                    None
                };

                Ok(Some(ast::Stmt::Return(ast::ReturnStmt {
                    span: DUMMY_SP,
                    arg,
                })))
            }
            Node::ExpressionStatement(expr) => {
                let swc_expr = self.node_to_swc_expr(expr.node);
                Ok(Some(ast::Stmt::Expr(ast::ExprStmt {
                    span: DUMMY_SP,
                    expr: Box::new(swc_expr),
                })))
            }
            _ => Ok(None),
        }
    }

    fn declaration_to_swc_statement(
        &mut self,
        node: Node,
    ) -> Result<Option<ast::Stmt>, Box<dyn Error>> {
        let Node::VariableDeclaration {
            name,
            op,
            initializer,
        } = node
        else {
            panic!("Variable declaration expected")
        };

        let init = initializer.map(|expr| Box::new(self.node_to_swc_expr(expr.node)));

        let decl = ast::VarDeclarator {
            span: DUMMY_SP,
            name: ast::Pat::Ident(ast::BindingIdent {
                id: create_ident(&name.unwrap()),
                type_ann: None,
            }),
            init,
            definite: false,
        };

        let kind = match op.as_str() {
            ":=" => ast::VarDeclKind::Let,
            "::" => ast::VarDeclKind::Const,
            _ => panic!("Unexpected declaration operator '{}'", op),
        };

        Ok(Some(ast::Stmt::Decl(ast::Decl::Var(Box::new(
            ast::VarDecl {
                span: DUMMY_SP,
                kind,
                declare: false,
                decls: vec![decl],
            },
        )))))
    }
}
