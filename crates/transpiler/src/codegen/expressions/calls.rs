use crate::codegen::{utils::create_ident, CodeGenerator};

use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;
use tine_core::ast;

impl CodeGenerator<'_> {
    pub fn call_expr_to_swc(&mut self, node: &ast::CallExpression) -> swc::CallExpr {
        match node.callee.as_ref().unwrap().as_ref() {
            ast::Expression::Identifier(id) if id.as_str() == "derived$" => {
                return self.derived_call_to_swc(&node);
            }
            _ => {}
        }

        let callee = swc::Callee::Expr(Box::new(self.expr_to_swc(node.callee.as_ref().unwrap())));
        let args = node
            .args
            .iter()
            .map(|arg| self.call_arg_to_swc(arg).into())
            .collect();
        swc::CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee,
            args,
            type_args: None,
        }
    }

    fn call_arg_to_swc(&mut self, node: &ast::CallArgument) -> swc::Expr {
        match node {
            ast::CallArgument::Expression(expr) => self.expr_to_swc(expr),
            ast::CallArgument::Callback(cb) => self.callback_to_swc(cb).into(),
        }
    }

    fn callback_to_swc(&mut self, node: &ast::Callback) -> swc::ArrowExpr {
        let params = node
            .params
            .iter()
            .map(|param| self.predicate_param_to_swc(&param))
            .collect();
        swc::ArrowExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            params,
            body: Box::new(match &**node.body.as_ref().unwrap() {
                ast::Expression::Block(b) => self.function_body_to_swc(b),
                expr => swc::BlockStmtOrExpr::Expr(Box::new(self.expr_to_swc(expr))),
            }),
            is_async: false,
            is_generator: false,
            type_params: None,
            return_type: None,
        }
    }

    fn predicate_param_to_swc(&mut self, node: &ast::CallbackParam) -> swc::Pat {
        let name = match node {
            ast::CallbackParam::Identifier(id) => id.as_str(),
            ast::CallbackParam::Param(param) => param.name.as_str(),
        };
        swc::Pat::Ident(swc::BindingIdent {
            id: create_ident(name),
            type_ann: None,
        })
    }

    fn derived_call_to_swc(&mut self, node: &ast::CallExpression) -> swc::CallExpr {
        let getter = swc::Expr::Arrow(swc::ArrowExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            params: Vec::new(),
            body: Box::new(
                self.expr_to_swc(node.args[0].as_expression().unwrap())
                    .into(),
            ),
            is_async: false,
            is_generator: false,
            type_params: None,
            return_type: None,
        });

        let dependencies = swc::Expr::Array(self.listener_deps_to_swc_array(node.loc));

        swc::CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: swc::Callee::Expr(Box::new(create_ident("derived$").into())),
            args: vec![getter.into(), dependencies.into()],
            type_args: None,
        }
    }
}
