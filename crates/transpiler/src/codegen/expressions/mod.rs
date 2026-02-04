mod ifs;
mod member;
mod unary;

use super::{
    utils::{create_ident, undefined},
    CodeGenerator,
};
use crate::codegen::utils::{can_block_be_inlined, create_block_stmt, create_number, AssignTo};
use rand::{distr::Alphanumeric, Rng};
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;
use tine_core::{ast, types};

impl CodeGenerator<'_> {
    pub fn expr_or_an_to_swc(&mut self, node: &ast::ExpressionOrAnonymous) -> swc::Expr {
        match node {
            ast::ExpressionOrAnonymous::Expression(node) => self.expr_to_swc(node),
            ast::ExpressionOrAnonymous::Struct(node) => self.anonymous_struct_to_swc(node).into(),
        }
    }

    pub fn expr_to_swc(&mut self, node: &ast::Expression) -> swc::Expr {
        match node {
            ast::Expression::Array(node) => self.array_to_swc(node).into(),
            ast::Expression::Binary(node) => self.binary_expression_to_swc_expr(node),
            ast::Expression::BooleanLiteral(node) => swc::Lit::Bool(swc::Bool {
                span: DUMMY_SP,
                value: node.value,
            })
            .into(),
            ast::Expression::Block(node) => self.block_expr_to_swc(node).into(),
            ast::Expression::Call(node) => self.call_expr_to_swc(node).into(),
            ast::Expression::CompositeLiteral(node) => self.composite_literal_to_swc_expr(node),
            ast::Expression::Empty => panic!("shouldn't have empty expressions at codegen step"),
            ast::Expression::Element(node) => self.element_expression_to_swc(node),
            ast::Expression::FloatLiteral(node) => swc::Expr::Lit(swc::Lit::Num(swc::Number {
                span: DUMMY_SP,
                value: *node.value,
                raw: None,
            })),
            ast::Expression::Function(node) => self.function_expression_to_swc(node).into(),
            ast::Expression::Identifier(node) => self.ident_to_swc(node).into(),
            ast::Expression::If(node) => self.if_to_swc_expr(node).into(),
            ast::Expression::IfDecl(node) => self.if_decl_to_swc_expr(node).into(),
            ast::Expression::Invalid(_) => {
                unreachable!("Invalid input should've been detected during analysis phase")
            }
            ast::Expression::IntLiteral(node) => swc::Expr::Lit(swc::Lit::Num(swc::Number {
                span: DUMMY_SP,
                value: node.value as f64,
                raw: None,
            })),
            ast::Expression::Loop(node) => self.loop_to_swc_expr(node).into(),
            ast::Expression::Match(node) => self.match_to_swc_expr(node).into(),
            ast::Expression::Member(node) => self.member_expr_to_swc(node).into(),
            ast::Expression::Unary(node) => self.unary_expression_to_swc_expr(node),
            ast::Expression::StringLiteral(node) => swc::Lit::Str(swc::Str {
                span: DUMMY_SP,
                value: node.as_str().into(),
                raw: None,
            })
            .into(),
            ast::Expression::Tuple(node) => self.tuple_to_swc(node).into(),
        }
    }

    pub fn array_to_swc(&mut self, node: &ast::ArrayExpression) -> swc::ArrayLit {
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

    fn binary_expression_to_swc_expr(&mut self, node: &ast::BinaryExpression) -> swc::Expr {
        let left_expr = self.expr_to_swc(&node.left);
        let right_expr = self.expr_to_swc(&node.right);

        let op = match node.operator {
            ast::BinaryOperator::Add => swc::BinaryOp::Add,
            ast::BinaryOperator::Div => swc::BinaryOp::Div,
            ast::BinaryOperator::EqEq => swc::BinaryOp::EqEqEq,
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

        let mut expr = swc::Expr::Bin(swc::BinExpr {
            span: DUMMY_SP,
            op,
            left: Box::new(left_expr),
            right: Box::new(right_expr),
        });
        if self.get_type_at(node.loc) == Some(types::Type::Integer) {
            expr = swc::Expr::Bin(swc::BinExpr {
                span: DUMMY_SP,
                op: swc::BinaryOp::BitOr,
                left: Box::new(expr),
                right: Box::new(swc::Expr::Lit(swc::Lit::Num(swc::Number {
                    span: DUMMY_SP,
                    value: 0.,
                    raw: None,
                }))),
            });
        }
        expr
    }

    fn block_expr_to_swc(&mut self, node: &ast::BlockExpression) -> swc::Expr {
        if node.statements.len() == 0 {
            undefined()
        } else if can_block_be_inlined(node) {
            self.block_to_swc_inlined(node).into()
        } else {
            self.block_to_swc_extracted(node).into()
        }
    }

    fn block_to_swc_inlined(&mut self, node: &ast::BlockExpression) -> swc::Expr {
        if node.statements.len() == 1 {
            let ast::Statement::Expression(ref expr) = node.statements[0] else {
                panic!()
            };
            return self.expr_to_swc(&expr.expression);
        }

        let exprs = node
            .statements
            .iter()
            .map(|stmt| {
                let ast::Statement::Expression(expr) = stmt else {
                    panic!()
                };
                Box::new(self.expr_to_swc(&expr.expression))
            })
            .collect();

        swc::Expr::Seq(swc::SeqExpr {
            span: DUMMY_SP,
            exprs,
        })
    }

    /// Extract a block from the current expression.
    /// For example:
    ///
    /// ```
    /// 42 + {
    ///     x := ...
    ///     x + 1
    /// }
    /// ```
    ///
    /// into:
    ///
    /// ```js
    /// let __cq6s81c68qzzej5i;
    /// {
    ///     let x = ...
    ///     __cq6s81c68qzzej5i = x + 1;
    /// }
    /// 42 + __cq6s81c68qzzej5i;
    /// ```
    fn block_to_swc_extracted(&mut self, node: &ast::BlockExpression) -> swc::Ident {
        let len = node.statements.len();
        assert!(len > 0);

        let id = self.add_temp_var_to_current_block();
        self.enter_block();
        let block = self.block_to_swc_stmt(node, AssignTo::Last(id.clone()));
        self.exit_block();
        self.push_to_block(block.into());
        create_ident(&id)
    }

    fn call_expr_to_swc(&mut self, node: &ast::CallExpression) -> swc::CallExpr {
        let callee = swc::Callee::Expr(Box::new(self.expr_to_swc(&node.callee)));
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
            body: Box::new(match &*node.body {
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

    fn function_expression_to_swc(&mut self, node: &ast::FunctionExpression) -> swc::ArrowExpr {
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

    pub fn function_params_to_swc(&mut self, params: &Vec<ast::FunctionParam>) -> Vec<swc::Pat> {
        params
            .into_iter()
            .map(|param| {
                swc::Pat::Ident(swc::BindingIdent {
                    id: create_ident(param.name.as_str()),
                    type_ann: None,
                })
            })
            .collect()
    }

    pub fn function_body_to_swc(&mut self, body: &ast::BlockExpression) -> swc::BlockStmtOrExpr {
        let stmts = body
            .statements
            .iter()
            .flat_map(|stmt| self.stmt_to_swc(stmt))
            .collect();

        swc::BlockStmtOrExpr::BlockStmt(create_block_stmt(stmts))
    }

    /// Create code for identifiers.
    /// Identifiers that have references are declared wrapped in an array (like `let identifier = [value]`), so their reads are generated like `identifier[0]`
    pub fn ident_to_swc(&mut self, node: &ast::Identifier) -> swc::Expr {
        let info = self.find_symbol(node.loc).unwrap();

        if info.borrow().has_ref() {
            swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(create_ident(node.as_str()).into()),
                prop: swc::MemberProp::Computed(swc::ComputedPropName {
                    span: DUMMY_SP,
                    expr: Box::new(create_number(0.0)),
                }),
            })
        } else {
            swc::Expr::Ident(create_ident(node.as_str()))
        }
    }

    fn loop_to_swc_expr(&mut self, node: &ast::Loop) -> swc::Expr {
        let id = self.add_temp_var_to_current_block();
        self.enter_block();
        let loop_stmt = self.loop_to_swc_stmt(node, AssignTo::Break(id.clone()));
        self.exit_block();
        self.push_to_block(loop_stmt.into());
        let option = self.into_option(&id);
        self.push_to_block(option);
        create_ident(&id).into()
    }

    fn match_to_swc_expr(&mut self, node: &ast::MatchExpression) -> swc::Expr {
        let id = self.add_temp_var_to_current_block();
        self.enter_block();
        let stmt = self.match_to_swc_stmt(node, AssignTo::Last(id.clone()));
        self.exit_block();
        self.push_to_block(stmt.into());
        create_ident(&id).into()
    }

    fn tuple_to_swc(&mut self, node: &ast::TupleExpression) -> swc::ArrayLit {
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

    pub fn add_temp_var_to_current_block(&mut self) -> String {
        let id = "__".to_string()
            + rand::rng()
                .sample_iter(&Alphanumeric)
                .take(16)
                .map(char::from)
                .collect::<String>()
                .as_str();

        self.push_to_block(swc::Stmt::Decl(swc::Decl::Var(Box::new(swc::VarDecl {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            kind: swc::VarDeclKind::Let,
            declare: false,
            decls: vec![swc::VarDeclarator {
                span: DUMMY_SP,
                name: swc::Pat::Ident(swc::BindingIdent {
                    id: create_ident(&id),
                    type_ann: None,
                }),
                init: Some(Box::new(undefined())),
                definite: false,
            }],
        }))));
        id
    }

    pub fn assignment_expression(&mut self, to: &str, expr: swc::Expr) -> swc::ExprStmt {
        swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(swc::Expr::Assign(swc::AssignExpr {
                span: DUMMY_SP,
                op: swc::AssignOp::Assign,
                left: swc::AssignTarget::Simple(swc::SimpleAssignTarget::Ident(
                    swc::BindingIdent {
                        id: create_ident(to),
                        type_ann: None,
                    },
                )),
                right: Box::new(expr),
            })),
        }
    }
}
