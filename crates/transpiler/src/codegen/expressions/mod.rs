mod dom;
mod ifs;
mod unary;
mod utils;

use super::{
    utils::{create_ident, undefined},
    CodeGenerator,
};
use crate::codegen::{
    expressions::utils::stmt_to_iife,
    utils::{can_block_be_inlined, create_block_stmt, create_number, create_str},
};
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;
use tine_core::{ir, TypeStore};

impl CodeGenerator<'_> {
    pub fn expr_to_swc(&mut self, node: &ir::Expression) -> swc::Expr {
        match node {
            ir::Expression::Array(a) => self.array_to_swc(a).into(),
            ir::Expression::Binary(b) => self.binary_expression_to_swc_expr(b).into(),
            ir::Expression::BooleanLiteral(b) => swc::Expr::Lit(swc::Lit::Bool(swc::Bool {
                span: DUMMY_SP,
                value: b.value,
            })),
            ir::Expression::Block(b) => self.block_expr_to_swc(b).into(),
            ir::Expression::Call(c) => self.call_expr_to_swc(c).into(),
            ir::Expression::Element(e) => self.element_expression_to_swc(e),
            ir::Expression::FloatLiteral(f) => swc::Expr::Lit(swc::Lit::Num(swc::Number {
                span: DUMMY_SP,
                value: f.value,
                raw: None,
            })),
            ir::Expression::For(f) => self.for_to_swc(f).into(),
            ir::Expression::ForIn(f) => self.for_in_to_swc(f).into(),
            ir::Expression::Function(f) => self.function_expression_to_swc(f).into(),
            ir::Expression::Identifier(i) => self.ident_to_swc(i).into(),
            ir::Expression::If(i) => self.if_to_swc_expr(i).into(),
            ir::Expression::IntLiteral(i) => swc::Expr::Lit(swc::Lit::Num(swc::Number {
                span: DUMMY_SP,
                value: i.value as f64,
                raw: None,
            })),
            ir::Expression::Map(m) => self.map_to_swc(m).into(),
            ir::Expression::Member(m) => self.member_expr_to_swc(m).into(),
            ir::Expression::StringLiteral(s) => self.string_literal_to_swc(s).into(),
            ir::Expression::Struct(s) => self.struct_to_swc(s).into(),
            ir::Expression::Unary(u) => self.unary_expression_to_swc_expr(u),
            ir::Expression::Tuple(t) => self.tuple_to_swc(t).into(),
            ir::Expression::TypeMatch(t) => self.type_match_to_swc(t).into(),
        }
    }

    pub fn array_to_swc(&mut self, node: &ir::ArrayExpression) -> swc::ArrayLit {
        let elems = node
            .elements
            .iter()
            .map(|e| Some(self.expr_to_swc(e).into()))
            .collect();

        swc::ArrayLit {
            span: DUMMY_SP,
            elems,
        }
    }

    fn binary_expression_to_swc_expr(&mut self, node: &ir::BinaryExpression) -> swc::BinExpr {
        let left_expr = self.expr_to_swc(&node.left);
        let right_expr = self.expr_to_swc(&node.right);

        let op = match node.op {
            ir::BinaryOperator::Add => swc::BinaryOp::Add,
            ir::BinaryOperator::Div => swc::BinaryOp::Div,
            ir::BinaryOperator::EqEq => swc::BinaryOp::EqEqEq,
            ir::BinaryOperator::Geq => swc::BinaryOp::GtEq,
            ir::BinaryOperator::Grt => swc::BinaryOp::Gt,
            ir::BinaryOperator::LAnd => swc::BinaryOp::LogicalAnd,
            ir::BinaryOperator::LOr => swc::BinaryOp::LogicalOr,
            ir::BinaryOperator::Leq => swc::BinaryOp::LtEq,
            ir::BinaryOperator::Less => swc::BinaryOp::Lt,
            ir::BinaryOperator::Mod => swc::BinaryOp::Mod,
            ir::BinaryOperator::Mul => swc::BinaryOp::Mul,
            ir::BinaryOperator::Neq => swc::BinaryOp::NotEqEq,
            ir::BinaryOperator::Pow => swc::BinaryOp::Exp,
            ir::BinaryOperator::Sub => swc::BinaryOp::Sub,
        };

        let mut expr = swc::BinExpr {
            span: DUMMY_SP,
            op,
            left: Box::new(left_expr),
            right: Box::new(right_expr),
        };
        if node.ty == TypeStore::INTEGER {
            expr = swc::BinExpr {
                span: DUMMY_SP,
                op: swc::BinaryOp::BitOr,
                left: Box::new(expr.into()),
                right: Box::new(swc::Expr::Lit(swc::Lit::Num(swc::Number {
                    span: DUMMY_SP,
                    value: 0.,
                    raw: None,
                }))),
            };
        }
        expr
    }

    fn block_expr_to_swc(&mut self, node: &ir::Block) -> swc::Expr {
        if node.statements.len() == 0 {
            undefined()
        } else if can_block_be_inlined(node) {
            self.block_to_swc_inlined(node).into()
        } else {
            self.block_to_iife(node).into()
        }
    }

    fn block_to_swc_inlined(&mut self, node: &ir::Block) -> swc::Expr {
        if node.statements.len() == 1 {
            let ir::Statement::Expression(expr) = &node.statements[0] else {
                panic!()
            };
            return self.expr_to_swc(expr);
        }

        let exprs = node
            .statements
            .iter()
            .map(|stmt| match stmt {
                ir::Statement::Assignment(a) => Box::new(self.assignment_to_swc_expr(a).into()),
                ir::Statement::Expression(expr) => Box::new(self.expr_to_swc(expr)),
                _ => panic!(),
            })
            .collect();

        swc::Expr::Seq(swc::SeqExpr {
            span: DUMMY_SP,
            exprs,
        })
    }

    fn block_to_iife(&mut self, node: &ir::Block) -> swc::Expr {
        swc::Expr::Call(swc::CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            args: vec![],
            callee: swc::Callee::Expr(Box::new(swc::Expr::Arrow(swc::ArrowExpr {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                params: vec![],
                body: Box::new(self.block_to_swc_stmt(node).into()),
                is_async: false,
                is_generator: false,
                type_params: None,
                return_type: None,
            }))),
            type_args: None,
        })
    }

    fn call_expr_to_swc(&mut self, node: &ir::CallExpression) -> swc::CallExpr {
        let callee = swc::Callee::Expr(Box::new(self.expr_to_swc(&node.callee)));
        let args = node
            .args
            .iter()
            .map(|arg| self.expr_to_swc(arg).into())
            .collect();
        swc::CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee,
            args,
            type_args: None,
        }
    }

    fn for_to_swc(&mut self, node: &ir::ForExpression) -> swc::Expr {
        self.with_breaks_to_returns(|self_| stmt_to_iife(self_.for_to_swc_stmt(node).into()))
    }

    fn for_in_to_swc(&mut self, node: &ir::ForInExpression) -> swc::Expr {
        self.with_breaks_to_returns(|self_| stmt_to_iife(self_.for_in_to_swc_stmt(node).into()))
    }

    fn function_expression_to_swc(&mut self, node: &ir::FunctionExpression) -> swc::ArrowExpr {
        let swc_params = self.function_params_to_swc(&node.params);
        let swc_body = self.function_body_to_swc(&node.body);

        swc::ArrowExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            params: swc_params,
            body: Box::new(swc_body),
            is_async: false,
            is_generator: false,
            type_params: None,
            return_type: None,
        }
    }

    pub fn function_params_to_swc(&mut self, params: &Vec<ir::Identifier>) -> Vec<swc::Pat> {
        params
            .into_iter()
            .map(|param| {
                swc::Pat::Ident(swc::BindingIdent {
                    id: create_ident(&param.as_name()),
                    type_ann: None,
                })
            })
            .collect()
    }

    pub fn function_body_to_swc(&mut self, body: &ir::Block) -> swc::BlockStmtOrExpr {
        let stmts = body
            .statements
            .iter()
            .map(|stmt| self.stmt_to_swc(stmt))
            .collect();

        swc::BlockStmtOrExpr::BlockStmt(create_block_stmt(stmts))
    }

    /// Create code for identifiers.
    /// Identifiers that have references are declared wrapped in an array (like `let identifier = [value]`), so their reads are generated like `identifier[0]`
    pub fn ident_to_swc(&mut self, node: &ir::Identifier) -> swc::Expr {
        let info = self.find_symbol(node.loc).unwrap();

        if info.borrow().has_ref() {
            swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(create_ident(&node.as_name()).into()),
                prop: swc::MemberProp::Computed(swc::ComputedPropName {
                    span: DUMMY_SP,
                    expr: Box::new(create_number(0.0)),
                }),
            })
        } else {
            swc::Expr::Ident(create_ident(&node.as_name()))
        }
    }

    fn map_to_swc(&mut self, node: &ir::MapLiteral) -> swc::Expr {
        swc::Expr::Object(swc::ObjectLit {
            span: DUMMY_SP,
            props: node
                .entries
                .iter()
                .map(|entry| self.map_entry_to_swc_prop(entry))
                .collect(),
        })
    }
    fn map_entry_to_swc_prop(&mut self, entry: &ir::MapEntry) -> swc::PropOrSpread {
        let key = match &entry.key {
            ir::Expression::StringLiteral(s) => swc::PropName::Str(self.string_literal_to_swc(s)),
            expr => swc::PropName::Computed(swc::ComputedPropName {
                span: DUMMY_SP,
                expr: Box::new(self.expr_to_swc(expr)),
            }),
        };
        let value = Box::new(self.expr_to_swc(&entry.value));
        swc::PropOrSpread::Prop(Box::new(swc::Prop::KeyValue(swc::KeyValueProp {
            key,
            value,
        })))
    }

    pub fn member_expr_to_swc(&mut self, node: &ir::MemberExpression) -> swc::MemberExpr {
        let prop_name = node.member.as_name();
        let prop = match prop_name.parse::<usize>() {
            Ok(int) => swc::MemberProp::Computed(swc::ComputedPropName {
                span: DUMMY_SP,
                expr: Box::new(swc::Expr::Lit(swc::Lit::Num(swc::Number {
                    span: DUMMY_SP,
                    value: int as f64,
                    raw: None,
                }))),
            }),
            Err(_) => swc::MemberProp::Ident(create_ident(&prop_name).into()),
        };

        swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(self.expr_to_swc(&node.object)),
            prop,
        }
    }

    fn string_literal_to_swc(&mut self, node: &ir::StringLiteral) -> swc::Str {
        swc::Str {
            span: DUMMY_SP,
            value: node.value.clone().into(),
            raw: None,
        }
    }

    fn struct_to_swc(&mut self, node: &ir::StructLiteral) -> swc::ObjectLit {
        let mut props = node
            .fields
            .iter()
            .map(|field| {
                swc::PropOrSpread::Prop(Box::new(swc::Prop::KeyValue(swc::KeyValueProp {
                    key: swc::PropName::Ident(create_ident(&field.name.as_name()).into()),
                    value: Box::new(self.expr_to_swc(&field.value)),
                })))
            })
            .collect::<Vec<_>>();
        if let Some(constructor) = &node.constructor {
            let tag = swc::PropOrSpread::Prop(Box::new(swc::Prop::KeyValue(swc::KeyValueProp {
                key: swc::PropName::Ident(create_ident("$tag").into()),
                value: Box::new(create_str(&constructor.as_name())),
            })));
            props.push(tag);
        }
        swc::ObjectLit {
            span: DUMMY_SP,
            props,
        }
    }

    fn tuple_to_swc(&mut self, node: &ir::TupleExpression) -> swc::ArrayLit {
        let elems = node
            .elements
            .iter()
            .map(|node| Some(self.expr_to_swc(node).into()))
            .collect::<Vec<_>>();
        swc::ArrayLit {
            span: DUMMY_SP,
            elems,
        }
    }

    fn type_match_to_swc(&mut self, node: &ir::TypeMatch) -> swc::BinExpr {
        swc::BinExpr {
            span: DUMMY_SP,
            op: swc::BinaryOp::EqEqEq,
            left: Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(self.expr_to_swc(&node.expr)),
                prop: swc::MemberProp::Ident(create_ident("$tag").into()),
            })),
            right: Box::new(create_str(&node.constructor.as_name())),
        }
    }
}
