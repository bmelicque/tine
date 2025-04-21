use std::error::Error;
use swc_common::DUMMY_SP;
use swc_ecma_ast as ast;

use crate::ast::Node;

use super::{
    expressions::node_to_swc_expr, type_declaration::type_declaration_to_swc_decl, CodeGenerator,
};

pub fn node_to_swc_stmt(
    generator: &CodeGenerator,
    node: Node,
) -> Result<Option<ast::Stmt>, Box<dyn Error>> {
    match node {
        Node::VariableDeclaration {
            name,
            op,
            initializer,
        } => {
            let init = if let Some(expr) = initializer {
                let swc_expr = node_to_swc_expr(generator, expr.node)?;
                Some(Box::new(swc_expr))
            } else {
                None
            };

            let decl = ast::VarDeclarator {
                span: DUMMY_SP,
                name: ast::Pat::Ident(ast::BindingIdent {
                    id: ast::Ident {
                        span: DUMMY_SP,
                        sym: name.unwrap().into(),
                        optional: false,
                    },
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
        Node::TypeDeclaration { .. } => type_declaration_to_swc_decl(generator, node),
        Node::Assignment { name, value } => {
            let value_expr = if let Some(v) = value {
                node_to_swc_expr(generator, v.node)?
            } else {
                panic!("Missing expression in assignment!");
            };

            let Some(name) = name else {
                panic!("Missing variable name in assignment.");
            };

            let swc_name = ast::Ident {
                span: DUMMY_SP,
                sym: name.into(),
                optional: false,
            };

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
                let swc_expr = node_to_swc_expr(generator, e.node)?;
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
            let swc_expr = node_to_swc_expr(generator, expr.node)?;
            Ok(Some(ast::Stmt::Expr(ast::ExprStmt {
                span: DUMMY_SP,
                expr: Box::new(swc_expr),
            })))
        }
        _ => Ok(None),
    }
}
