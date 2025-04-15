use std::error::Error;

use swc_common::DUMMY_SP;
use swc_ecma_ast::{
    ArrowExpr, BinExpr, BinaryOp, BindingIdent, BlockStmt, BlockStmtOrExpr, Bool, Expr, Ident, Lit,
    Number, Pat, ReturnStmt, Stmt, Str,
};

use crate::ast::Node;

use super::{codegen::TranspilerError, CodeGenerator};

pub fn node_to_swc_expr(generator: &CodeGenerator, node: Node) -> Result<Expr, Box<dyn Error>> {
    match node {
        Node::FunctionExpression {
            parameters,
            return_type: _,
            body,
        } => {
            // Convert parameters
            let swc_params = parameters
                .unwrap_or_default()
                .into_iter()
                .map(|param| {
                    Pat::Ident(BindingIdent {
                        id: Ident {
                            span: DUMMY_SP,
                            sym: param.name.into(),
                            optional: false,
                        },
                        type_ann: None,
                    })
                })
                .collect();

            let swc_body = if let Some(body_node) = body {
                match &body_node.node {
                    Node::Block(statements) => {
                        // Convert block statement
                        let mut stmts = vec![];
                        for stmt in statements {
                            let expr = node_to_swc_expr(generator, stmt.node.clone())?;
                            stmts.push(Stmt::Return(ReturnStmt {
                                span: DUMMY_SP,
                                arg: Some(Box::new(expr)),
                            }));
                        }

                        BlockStmtOrExpr::BlockStmt(BlockStmt {
                            span: DUMMY_SP,
                            stmts,
                        })
                    }
                    _ => {
                        // Expression-style body
                        let expr = node_to_swc_expr(generator, body_node.node.clone())?;
                        BlockStmtOrExpr::Expr(Box::new(expr))
                    }
                }
            } else {
                return Err(Box::new(TranspilerError {
                    message: "Function body is missing".to_string(),
                }));
            };

            Ok(Expr::Arrow(ArrowExpr {
                span: DUMMY_SP,
                params: swc_params,
                body: Box::new(swc_body),
                is_async: false,
                is_generator: false,
                type_params: None,
                return_type: None,
            }))
        }
        Node::BinaryExpression {
            left,
            operator,
            right,
        } => {
            let left_expr = node_to_swc_expr(generator, left.unwrap().node)?;
            let right_expr = node_to_swc_expr(generator, right.unwrap().node)?;

            let op = match operator.as_str() {
                "+" => BinaryOp::Add,
                "-" => BinaryOp::Sub,
                "*" => BinaryOp::Mul,
                "/" => BinaryOp::Div,
                "==" => BinaryOp::EqEq,
                "!=" => BinaryOp::NotEq,
                "<" => BinaryOp::Lt,
                ">" => BinaryOp::Gt,
                "<=" => BinaryOp::LtEq,
                ">=" => BinaryOp::GtEq,
                _ => {
                    return Err(Box::new(TranspilerError {
                        message: format!("Unknown operator: {}", operator),
                    }))
                }
            };

            Ok(Expr::Bin(BinExpr {
                span: DUMMY_SP,
                op,
                left: Box::new(left_expr),
                right: Box::new(right_expr),
            }))
        }
        Node::Identifier(name) => Ok(Expr::Ident(Ident {
            span: DUMMY_SP,
            sym: name.into(),
            optional: false,
        })),
        Node::StringLiteral(value) => Ok(Expr::Lit(Lit::Str(Str {
            span: DUMMY_SP,
            value: value.into(),
            raw: None,
        }))),
        Node::NumberLiteral(value) => Ok(Expr::Lit(Lit::Num(Number {
            span: DUMMY_SP,
            value,
            raw: None,
        }))),
        Node::BooleanLiteral(value) => Ok(Expr::Lit(Lit::Bool(Bool {
            span: DUMMY_SP,
            value,
        }))),
        _ => Err(Box::new(TranspilerError {
            message: format!("Unsupported expression: {:?}", node),
        })),
    }
}
