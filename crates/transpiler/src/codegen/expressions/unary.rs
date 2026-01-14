use crate::codegen::{
    utils::{create_ident, create_number, create_str},
    CodeGenerator,
};
use tine_core::{ast, Location};
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;

impl CodeGenerator<'_> {
    pub fn unary_expression_to_swc_expr(&mut self, node: &ast::UnaryExpression) -> swc::Expr {
        match node.operator {
            ast::UnaryOperator::Ampersand => self.ref_to_swc_expr(node),
            ast::UnaryOperator::At => self.listener_to_swc_expr(node),
            ast::UnaryOperator::Bang => self.logical_not_to_swc_expr(node),
            ast::UnaryOperator::Dollar => self.signal_to_swc_expr(node),
            ast::UnaryOperator::Minus => self.negation_to_swc_expr(node),
            ast::UnaryOperator::Star => self.indirection_to_swc_expr(node),
        }
    }

    /**
     * `*expr` => `expr.get()`
     */
    fn indirection_to_swc_expr(&mut self, node: &ast::UnaryExpression) -> swc::Expr {
        swc::Expr::Call(swc::CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: swc::Callee::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(self.expr_to_swc(&node.operand)),
                prop: swc::MemberProp::Ident(create_ident("get").into()),
            }))),
            args: vec![],
            type_args: None,
        })
    }

    fn ref_to_swc_expr(&mut self, node: &ast::UnaryExpression) -> swc::Expr {
        let (ctx, value) = match node.operand.as_ref() {
            ast::Expression::Identifier(expr) => {
                (self.ident_to_swc(expr).into(), create_number(0.0))
            }
            ast::Expression::Member(expr) => (
                self.expr_to_swc(&expr.object),
                match expr.prop.clone().unwrap() {
                    ast::MemberProp::FieldName(i) => create_str(i.as_str().into()),
                    ast::MemberProp::Index(n) => create_number(*n.value),
                },
            ),
            expr => (
                swc::Expr::Ident(create_ident("undefined")),
                self.expr_to_swc(expr),
            ),
        };
        swc::Expr::New(swc::NewExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Box::new(swc::Expr::Ident(create_ident("__Reference"))),
            args: Some(vec![ctx.into(), value.into()]),
            type_args: None,
        })
    }

    fn signal_to_swc_expr(&mut self, node: &ast::UnaryExpression) -> swc::Expr {
        let init = self.expr_to_swc(&node.operand);
        swc::Expr::New(swc::NewExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(swc::Expr::Ident(create_ident("__"))),
                prop: swc::MemberProp::Ident(create_ident("Signal").into()),
            })),
            args: Some(vec![init.into()]),
            type_args: None,
        })
    }

    fn listener_to_swc_expr(&mut self, node: &ast::UnaryExpression) -> swc::Expr {
        let getter = self.listener_expr_to_swc_getter(&node.operand);
        let dependencies = swc::Expr::Array(self.listener_deps_to_swc_array(node.loc));

        swc::Expr::New(swc::NewExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(swc::Expr::Ident(create_ident("__"))),
                prop: swc::MemberProp::Ident(create_ident("Listener").into()),
            })),
            args: Some(vec![dependencies.into(), swc::Expr::Arrow(getter).into()]),
            type_args: None,
        })
    }

    fn listener_expr_to_swc_getter(&mut self, getter: &ast::Expression) -> swc::ArrowExpr {
        swc::ArrowExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            params: Vec::new(),
            body: Box::new(self.expr_to_swc(getter).into()),
            is_async: false,
            is_generator: false,
            type_params: None,
            return_type: None,
        }
    }

    pub fn listener_deps_to_swc_array(&mut self, listener_loc: Location) -> swc::ArrayLit {
        let reactive_dependencies: Vec<swc::Ident> = self
            .get_reactive_dependencies(listener_loc)
            .into_iter()
            .map(|dep| create_ident(&dep.borrow().name))
            .collect();
        swc::ArrayLit {
            span: DUMMY_SP,
            elems: reactive_dependencies
                .into_iter()
                .map(|dep| Some(swc::Expr::Ident(dep).into()))
                .collect(),
        }
    }

    fn negation_to_swc_expr(&mut self, node: &ast::UnaryExpression) -> swc::Expr {
        swc::Expr::Unary(swc::UnaryExpr {
            span: DUMMY_SP,
            op: swc::UnaryOp::Minus,
            arg: Box::new(self.expr_to_swc(&node.operand)),
        })
    }

    fn logical_not_to_swc_expr(&mut self, node: &ast::UnaryExpression) -> swc::Expr {
        swc::Expr::Unary(swc::UnaryExpr {
            span: DUMMY_SP,
            op: swc::UnaryOp::Bang,
            arg: Box::new(self.expr_to_swc(&node.operand)),
        })
    }
}
