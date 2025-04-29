use swc_common::DUMMY_SP;
use swc_ecma_ast::{
    ArrowExpr, BinExpr, BinaryOp, BindingIdent, BlockStmt, BlockStmtOrExpr, Bool, Expr, Ident, Lit,
    Number, Pat, ReturnStmt, Stmt, Str,
};

use crate::ast::Node;

use super::{utils::create_ident, CodeGenerator};

impl CodeGenerator {
    pub fn node_to_swc_expr(&mut self, node: Node) -> Expr {
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
                                let expr = self.node_to_swc_expr(stmt.node.clone());
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
                            let expr = self.node_to_swc_expr(body_node.node.clone());
                            BlockStmtOrExpr::Expr(Box::new(expr))
                        }
                    }
                } else {
                    panic!("Function body is missing");
                };

                Expr::Arrow(ArrowExpr {
                    span: DUMMY_SP,
                    params: swc_params,
                    body: Box::new(swc_body),
                    is_async: false,
                    is_generator: false,
                    type_params: None,
                    return_type: None,
                })
            }
            Node::BinaryExpression {
                left,
                operator,
                right,
            } => self.binary_expression_to_swc_expr(
                left.unwrap().node,
                operator,
                right.unwrap().node,
            ),

            Node::MapLiteral { entries, .. } => self.map_literal_to_swc_new_map(entries).into(),
            Node::ArrayLiteral { elements, .. } | Node::AnonymousArrayLiteral(elements) => {
                self.array_literal_to_swc_array(elements).into()
            }
            Node::OptionLiteral { value, .. } => {
                self.option_literal_to_swc_new_option(value).into()
            }
            Node::StructLiteral {
                struct_type,
                fields,
            } => self
                .struct_literal_to_swc_new_expr(struct_type.node, fields)
                .into(),

            Node::Identifier(name) => create_ident(&name).into(),
            Node::StringLiteral(value) => Expr::Lit(Lit::Str(Str {
                span: DUMMY_SP,
                value: value.into(),
                raw: None,
            })),
            Node::NumberLiteral(value) => Expr::Lit(Lit::Num(Number {
                span: DUMMY_SP,
                value,
                raw: None,
            })),
            Node::BooleanLiteral(value) => Expr::Lit(Lit::Bool(Bool {
                span: DUMMY_SP,
                value,
            })),
            _ => panic!("Unsupported node type for expression: {:?}", node),
        }
    }

    fn binary_expression_to_swc_expr(&mut self, left: Node, operator: String, right: Node) -> Expr {
        let left_expr = self.node_to_swc_expr(left);
        let right_expr = self.node_to_swc_expr(right);

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
            _ => panic!("Unsupported binary operator: {}", operator),
        };

        Expr::Bin(BinExpr {
            span: DUMMY_SP,
            op,
            left: Box::new(left_expr),
            right: Box::new(right_expr),
        })
    }
}
