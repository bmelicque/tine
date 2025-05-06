use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

use crate::ast;

use super::CodeGenerator;

impl CodeGenerator {
    pub fn expr_or_an_to_swc(&mut self, node: ast::ExpressionOrAnonymous) -> swc::Expr {
        match node {
            ast::ExpressionOrAnonymous::Array(node) => self.anonymous_array_to_swc(node).into(),
            ast::ExpressionOrAnonymous::Expression(node) => self.expr_to_swc(node),
            ast::ExpressionOrAnonymous::Struct(node) => self.anonymous_struct_to_swc(node).into(),
        }
    }

    pub fn expr_to_swc(&mut self, node: ast::Expression) -> swc::Expr {
        match node {
            ast::Expression::Binary(node) => self.binary_expression_to_swc_expr(node),
            ast::Expression::BooleanLiteral(node) => swc::Lit::Bool(swc::Bool {
                span: DUMMY_SP,
                value: node.value,
            })
            .into(),
            ast::Expression::CompositeLiteral(node) => self.composite_literal_to_swc_expr(node),
            ast::Expression::Empty => panic!("shouldn't have empty expressions at codegen step"),
            ast::Expression::FieldAccess(node) => self.field_access_to_swc(node).into(),
            ast::Expression::Function(node) => self.function_expression_to_swc(node).into(),
            ast::Expression::Identifier(node) => self.ident_to_swc(node).into(),
            ast::Expression::NumberLiteral(node) => swc::Lit::Num(swc::Number {
                span: DUMMY_SP,
                value: node.value,
                raw: None,
            })
            .into(),
            ast::Expression::StringLiteral(node) => swc::Lit::Str(swc::Str {
                span: DUMMY_SP,
                value: node.as_str().into(),
                raw: None,
            })
            .into(),
            ast::Expression::Tuple(node) => self.tuple_to_swc(node).into(),
            ast::Expression::TupleIndexing(node) => self.tuple_indexing_to_swc(node).into(),
        }
    }

    fn binary_expression_to_swc_expr(&mut self, node: ast::BinaryExpression) -> swc::Expr {
        let left_expr = self.expr_to_swc(*node.left);
        let right_expr = self.expr_to_swc(*node.right);

        let op = match node.operator {
            ast::BinaryOperator::Add => swc::BinaryOp::Add,
            ast::BinaryOperator::Div => swc::BinaryOp::Div,
            ast::BinaryOperator::Eq => swc::BinaryOp::EqEqEq,
            ast::BinaryOperator::Geq => swc::BinaryOp::GtEq,
            ast::BinaryOperator::Grt => swc::BinaryOp::Gt,
            ast::BinaryOperator::LAnd => swc::BinaryOp::LogicalAnd,
            ast::BinaryOperator::LOr => swc::BinaryOp::LogicalOr,
            ast::BinaryOperator::Leq => swc::BinaryOp::LtEq,
            ast::BinaryOperator::Less => swc::BinaryOp::Lt,
            ast::BinaryOperator::Mod => swc::BinaryOp::Mod,
            ast::BinaryOperator::Mul => swc::BinaryOp::Mul,
            ast::BinaryOperator::Neq => swc::BinaryOp::NotEqEq,
            ast::BinaryOperator::Pow => swc::BinaryOp::Exp,
            ast::BinaryOperator::Sub => swc::BinaryOp::Sub,
        };

        swc::BinExpr {
            span: DUMMY_SP,
            op,
            left: Box::new(left_expr),
            right: Box::new(right_expr),
        }
        .into()
    }

    fn field_access_to_swc(&mut self, node: ast::FieldAccessExpression) -> swc::MemberExpr {
        swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(self.expr_to_swc(*node.object)),
            prop: swc::MemberProp::Ident(self.ident_to_swc(node.prop)),
        }
    }

    fn function_expression_to_swc(&mut self, node: ast::FunctionExpression) -> swc::ArrowExpr {
        let swc_params = node
            .params
            .into_iter()
            .map(|param| {
                swc::Pat::Ident(swc::BindingIdent {
                    id: swc::Ident {
                        span: DUMMY_SP,
                        sym: param.name.as_str().into(),
                        optional: false,
                    },
                    type_ann: None,
                })
            })
            .collect();

        let swc_body = match &node.body {
            ast::FunctionBody::TypedBlock(typed_block) => {
                let stmts = typed_block
                    .block
                    .statements
                    .iter()
                    .filter_map(|stmt| self.stmt_to_swc(stmt.clone()))
                    .collect();

                swc::BlockStmtOrExpr::BlockStmt(swc::BlockStmt {
                    span: DUMMY_SP,
                    stmts,
                })
            }
            ast::FunctionBody::Expression(expr) => {
                swc::BlockStmtOrExpr::Expr(Box::new(self.expr_to_swc(*expr.clone())))
            }
        };

        swc::ArrowExpr {
            span: DUMMY_SP,
            params: swc_params,
            body: Box::new(swc_body),
            is_async: false,
            is_generator: false,
            type_params: None,
            return_type: None,
        }
    }

    pub fn ident_to_swc(&mut self, node: ast::Identifier) -> swc::Ident {
        swc::Ident {
            span: DUMMY_SP,
            sym: node.as_str().into(),
            optional: false,
        }
    }

    fn tuple_to_swc(&mut self, node: ast::TupleExpression) -> swc::ArrayLit {
        let elems = node
            .elements
            .into_iter()
            .map(|node| Some(self.expr_to_swc(node).into()))
            .collect::<Vec<_>>();
        swc::ArrayLit {
            span: DUMMY_SP,
            elems,
        }
    }

    fn tuple_indexing_to_swc(&mut self, node: ast::TupleIndexingExpression) -> swc::MemberExpr {
        swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(self.expr_to_swc(*node.tuple)),
            prop: swc::MemberProp::Computed(swc::ComputedPropName {
                span: DUMMY_SP,
                expr: Box::new(self.expr_to_swc(node.index.into())),
            }),
        }
    }
}
