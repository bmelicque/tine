use std::error::Error;

use swc_common::DUMMY_SP;
use swc_ecma_ast::{BinExpr, BinaryOp, Bool, Expr, Ident, Lit, Number, Str};

use crate::ast::Node;

use super::{codegen::TranspilerError, CodeGenerator};

pub fn node_to_swc_expr(generator: &CodeGenerator, node: Node) -> Result<Expr, Box<dyn Error>> {
    match node {
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
