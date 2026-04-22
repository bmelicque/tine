mod binary;
mod block;
mod dom;
mod ifs;
mod unary;
mod utils;

use super::{utils::ident_from_str, CodeGenerator};
use crate::codegen::utils::{create_block_stmt, create_str};
use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use tine_core::{ir, SymbolRef, TypeSymbolBody};

pub struct ExpressionResult {
    /// All the the statements needed to be ran before actually evaluating the expression
    pub prelim_stmts: Vec<swc::Stmt>,
    /// The actual expression
    pub expr: swc::Expr,
}
impl<T> From<T> for ExpressionResult
where
    T: Into<swc::Expr>,
{
    fn from(value: T) -> Self {
        Self {
            prelim_stmts: vec![],
            expr: value.into(),
        }
    }
}

impl CodeGenerator<'_> {
    pub fn handle_expression(&mut self, node: &ir::Expression) -> ExpressionResult {
        match node {
            ir::Expression::Array(a) => self.handle_array(&a.elements),
            ir::Expression::Binary(b) => self.handle_binary_expression(b),
            ir::Expression::BooleanLiteral(b) => ExpressionResult::from(swc::Bool {
                span: DUMMY_SP,
                value: b.value,
            }),
            ir::Expression::Block(b) => self.handle_block(b),
            ir::Expression::Call(c) => self.handle_call(c),
            ir::Expression::Element(e) => self.handle_element_expression(e),
            ir::Expression::FloatLiteral(f) => ExpressionResult::from(swc::Number {
                span: DUMMY_SP,
                value: f.value,
                raw: None,
            }),
            ir::Expression::For(f) => self.handle_for_expression(f),
            ir::Expression::ForIn(f) => self.handle_for_in_expression(f),
            ir::Expression::Function(f) => self.handle_function_expression(f).into(),
            ir::Expression::Identifier(i) => self.handle_identifier(i).into(),
            ir::Expression::If(i) => self.handle_if_expression(i),
            ir::Expression::IntLiteral(i) => ExpressionResult::from(swc::Number {
                span: DUMMY_SP,
                value: i.value as f64,
                raw: None,
            }),
            ir::Expression::Map(m) => self.handle_map_expression(m),
            ir::Expression::Member(m) => self.member_expr_to_swc(m),
            ir::Expression::StringLiteral(s) => self.string_literal_to_swc(s).into(),
            ir::Expression::Struct(s) => self.struct_to_swc(s),
            ir::Expression::Unary(u) => self.handle_unary_expression(u),
            ir::Expression::Tuple(t) => self.handle_array(&t.elements),
            ir::Expression::TypeMatch(t) => self.handle_type_match(t),
        }
    }

    pub fn handle_array(&mut self, elements: &Vec<ir::Expression>) -> ExpressionResult {
        let elements = elements
            .iter()
            .map(|e| self.handle_expression(e))
            .collect::<Vec<_>>();
        let (prelim_stmts, elems) = self.extract_necessary(elements);
        let expr = swc::Expr::Array(swc::ArrayLit {
            span: DUMMY_SP,
            elems: elems.into_iter().map(|e| Some(e.into())).collect(),
        });
        ExpressionResult { prelim_stmts, expr }
    }

    pub fn handle_call(&mut self, node: &ir::CallExpression) -> ExpressionResult {
        let mut callee_result = self.handle_expression(&node.callee);
        let args_results = node
            .args
            .iter()
            .map(|a| self.handle_expression(a))
            .collect::<Vec<_>>();
        let (prelim_stmts, args) = self.extract_necessary(args_results);
        if !prelim_stmts.is_empty() {
            callee_result = self.to_extracted(callee_result);
        }
        let prelim_stmts = vec![callee_result.prelim_stmts, prelim_stmts].concat();
        let callee = callee_result.expr;

        let expr = swc::Expr::Call(swc::CallExpr {
            callee: swc::Callee::Expr(Box::new(callee)),
            args: args.into_iter().map(Into::into).collect(),
            ..Default::default()
        });

        ExpressionResult { prelim_stmts, expr }
    }

    fn handle_for_expression(&mut self, node: &ir::ForExpression) -> ExpressionResult {
        let temp = self.get_temp_id();
        self.with_break_target(temp.clone(), |self_| ExpressionResult {
            prelim_stmts: self_.for_to_swc_stmt(node),
            expr: temp.into(),
        })
    }

    fn handle_for_in_expression(&mut self, node: &ir::ForInExpression) -> ExpressionResult {
        let temp = self.get_temp_id();
        self.with_break_target(temp.clone(), |self_| ExpressionResult {
            prelim_stmts: self_.for_in_to_swc_stmt(node),
            expr: temp.into(),
        })
    }

    fn handle_function_expression(&mut self, node: &ir::FunctionExpression) -> swc::ArrowExpr {
        let swc_params = self.function_params_to_swc(&node.params);
        let swc_body = self.function_body_to_swc(&node.body);

        swc::ArrowExpr {
            params: swc_params,
            body: Box::new(swc_body),
            ..Default::default()
        }
    }

    pub fn function_params_to_swc(&mut self, params: &Vec<ir::Identifier>) -> Vec<swc::Pat> {
        params
            .into_iter()
            .map(|param| {
                swc::Pat::Ident(swc::BindingIdent {
                    id: ident_from_str(&param.as_name()),
                    type_ann: None,
                })
            })
            .collect()
    }

    pub fn function_body_to_swc(&mut self, body: &ir::Block) -> swc::BlockStmtOrExpr {
        let stmts = body
            .statements
            .iter()
            .flat_map(|stmt| self.stmt_to_swc(stmt))
            .collect();

        swc::BlockStmtOrExpr::BlockStmt(create_block_stmt(stmts))
    }

    pub fn handle_identifier(&mut self, node: &ir::Identifier) -> swc::Expr {
        swc::Expr::Ident(ident_from_str(&node.as_name()))
    }

    fn handle_map_expression(&mut self, node: &ir::MapLiteral) -> ExpressionResult {
        let mut results = Vec::with_capacity(node.entries.len() * 2);
        for entry in &node.entries {
            results.push(self.handle_expression(&entry.key));
            results.push(self.handle_expression(&entry.value));
        }
        let (prelim_stmts, exprs) = self.extract_necessary(results);

        let mut props = Vec::with_capacity(node.entries.len());
        let mut iter = exprs.into_iter();
        while let (Some(key), Some(value)) = (iter.next(), iter.next()) {
            props.push(self.make_prop(key, value));
        }

        let expr = swc::Expr::Object(swc::ObjectLit {
            span: DUMMY_SP,
            props,
        });

        ExpressionResult { prelim_stmts, expr }
    }
    fn make_prop(&self, key: swc::Expr, value: swc::Expr) -> swc::PropOrSpread {
        swc::PropOrSpread::Prop(Box::new(swc::Prop::KeyValue(swc::KeyValueProp {
            key: swc::PropName::Computed(swc::ComputedPropName {
                span: DUMMY_SP,
                expr: Box::new(key),
            }),
            value: Box::new(value),
        })))
    }

    pub fn member_expr_to_swc(&mut self, node: &ir::MemberExpression) -> ExpressionResult {
        let obj_result = self.handle_expression(&node.object);

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
            Err(_) => swc::MemberProp::Ident(ident_from_str(&prop_name).into()),
        };

        let expr = swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(obj_result.expr),
            prop,
        };

        ExpressionResult {
            prelim_stmts: obj_result.prelim_stmts,
            expr: expr.into(),
        }
    }

    fn string_literal_to_swc(&mut self, node: &ir::StringLiteral) -> swc::Str {
        swc::Str {
            span: DUMMY_SP,
            value: node.value.clone().into(),
            raw: None,
        }
    }

    fn struct_to_swc(&mut self, node: &ir::StructLiteral) -> ExpressionResult {
        match &node.variant {
            Some(v) => self.handle_variant_struct(node, v),
            None => self.handle_simple_struct(node),
        }
    }

    fn handle_variant_struct(
        &mut self,
        node: &ir::StructLiteral,
        variant: &SymbolRef,
    ) -> ExpressionResult {
        let (prelim_stmts, args) = match variant.as_type_body() {
            Some(TypeSymbolBody::Struct(fields)) => self.handle_struct_like_body(node, fields),
            Some(TypeSymbolBody::Tuple(_)) => self.handle_tuple_like_body(node),
            None => (vec![], vec![]),
        };
        let callee = swc::Expr::Member(swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(swc::Expr::Ident(ident_from_str(
                &node.constructor.as_name(),
            ))),
            prop: swc::MemberProp::Ident(ident_from_str(&variant.as_name()).into()),
        });
        let expr = swc::Expr::Call(swc::CallExpr {
            callee: swc::Callee::Expr(Box::new(callee)),
            args: args.into_iter().map(Into::into).collect(),
            ..Default::default()
        });
        ExpressionResult { prelim_stmts, expr }
    }

    fn handle_simple_struct(&mut self, node: &ir::StructLiteral) -> ExpressionResult {
        let expected_body = node.constructor.as_type_body().unwrap();
        let (prelim_stmts, args) = match expected_body {
            TypeSymbolBody::Struct(fields) => self.handle_struct_like_body(node, fields),
            TypeSymbolBody::Tuple(_) => self.handle_tuple_like_body(node),
        };
        let expr = swc::Expr::New(swc::NewExpr {
            callee: Box::new(ident_from_str(&node.constructor.as_name()).into()),
            args: Some(args.into_iter().map(Into::into).collect()),
            ..Default::default()
        });

        ExpressionResult { prelim_stmts, expr }
    }

    fn handle_struct_like_body(
        &mut self,
        node: &ir::StructLiteral,
        expected: Vec<(String, SymbolRef)>,
    ) -> (Vec<swc::Stmt>, Vec<swc::Expr>) {
        let results = node
            .fields
            .iter()
            .map(|field| self.handle_expression(&field.value))
            .collect::<Vec<_>>();
        let (prelim, mut args) = self.extract_all(results);
        let mut order = expected
            .into_iter()
            .map(|(name, _)| {
                node.fields
                    .iter()
                    .enumerate()
                    .find(|(_, f)| f.name.symbol.borrow().name == name)
                    .unwrap()
                    .0
            })
            .collect::<Vec<_>>();
        for i in 0..args.len() {
            while order[i] != i {
                let next = order[i];
                args.swap(i, next);
                order.swap(i, next);
            }
        }
        (prelim, args)
    }

    fn handle_tuple_like_body(
        &mut self,
        node: &ir::StructLiteral,
    ) -> (Vec<swc::Stmt>, Vec<swc::Expr>) {
        let results = node
            .fields
            .iter()
            .map(|field| self.handle_expression(&field.value))
            .collect::<Vec<_>>();
        self.extract_necessary(results)
    }

    fn handle_type_match(&mut self, node: &ir::TypeMatch) -> ExpressionResult {
        let obj_result = self.handle_expression(&node.expr);

        let expr = swc::BinExpr {
            span: DUMMY_SP,
            op: swc::BinaryOp::EqEqEq,
            left: Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(obj_result.expr),
                prop: swc::MemberProp::Ident(ident_from_str("$tag").into()),
            })),
            right: Box::new(create_str(&node.constructor.as_name())),
        };

        ExpressionResult {
            prelim_stmts: obj_result.prelim_stmts,
            expr: expr.into(),
        }
    }
}
