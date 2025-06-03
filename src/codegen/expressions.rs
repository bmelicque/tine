use rand::{distr::Alphanumeric, Rng};
use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

use crate::{ast, codegen::utils::AssignTo};

use super::{
    utils::{create_ident, undefined},
    CodeGenerator,
};

impl CodeGenerator {
    pub fn expr_or_an_to_swc(&mut self, node: ast::ExpressionOrAnonymous) -> swc::Expr {
        match node {
            ast::ExpressionOrAnonymous::Expression(node) => self.expr_to_swc(node),
            ast::ExpressionOrAnonymous::Struct(node) => self.anonymous_struct_to_swc(node).into(),
        }
    }

    pub fn expr_to_swc(&mut self, node: ast::Expression) -> swc::Expr {
        match node {
            ast::Expression::Array(node) => self.array_to_swc(node).into(),
            ast::Expression::Binary(node) => self.binary_expression_to_swc_expr(node),
            ast::Expression::BooleanLiteral(node) => swc::Lit::Bool(swc::Bool {
                span: DUMMY_SP,
                value: node.value,
            })
            .into(),
            ast::Expression::Block(node) => self.block_expr_to_swc(node).into(),
            ast::Expression::CompositeLiteral(node) => self.composite_literal_to_swc_expr(node),
            ast::Expression::Empty => panic!("shouldn't have empty expressions at codegen step"),
            ast::Expression::FieldAccess(node) => self.field_access_to_swc(node).into(),
            ast::Expression::Function(node) => self.function_expression_to_swc(node).into(),
            ast::Expression::Identifier(node) => self.ident_to_swc(node).into(),
            ast::Expression::If(node) => self.if_to_swc_expr(node).into(),
            ast::Expression::IfDecl(node) => self.if_decl_to_swc_expr(node).into(),
            ast::Expression::Loop(node) => self.loop_to_swc_expr(node).into(),
            ast::Expression::Match(node) => self.match_to_swc_expr(node).into(),
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

    pub fn array_to_swc(&mut self, node: ast::ArrayExpression) -> swc::ArrayLit {
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

    fn block_expr_to_swc(&mut self, node: ast::BlockExpression) -> swc::Expr {
        if node.statements.len() == 0 {
            undefined()
        } else if node.can_be_inlined() {
            self.block_to_swc_inlined(node).into()
        } else {
            self.block_to_swc_extracted(node).into()
        }
    }

    fn block_to_swc_inlined(&mut self, node: ast::BlockExpression) -> swc::SeqExpr {
        let exprs = node
            .statements
            .iter()
            .map(|stmt| {
                let ast::Statement::Expression(expr) = stmt else {
                    panic!()
                };
                Box::new(self.expr_to_swc(*expr.expression.clone()))
            })
            .collect();
        swc::SeqExpr {
            span: DUMMY_SP,
            exprs,
        }
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
    fn block_to_swc_extracted(&mut self, node: ast::BlockExpression) -> swc::Ident {
        let len = node.statements.len();
        assert!(len > 0);

        let id = self.add_temp_var_to_current_block();
        self.enter_block();
        let block = self.block_to_swc_stmt(node, AssignTo::Last(id.clone()));
        self.exit_block();
        self.push_to_block(block.into());
        create_ident(&id)
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

    fn if_to_swc_expr(&mut self, node: ast::IfExpression) -> swc::Expr {
        if node.consequent.statements.len() == 0 && node.alternate.is_none() {
            undefined()
        } else if node.can_be_inlined() {
            self.if_to_swc_inlined(node).into()
        } else {
            self.if_to_swc_extracted(node).into()
        }
    }

    fn if_to_swc_inlined(&mut self, node: ast::IfExpression) -> swc::Expr {
        if let Some(alternate) = node.alternate {
            let alt = match *alternate {
                ast::Alternate::Block(b) => self.block_to_swc_inlined(b).into(),
                ast::Alternate::If(i) => self.if_to_swc_inlined(i).into(),
                ast::Alternate::IfDecl(_) => {
                    panic!("Shouldn't try to inline IfDeclExpression!")
                }
            };
            let alt = Box::new(alt);
            swc::Expr::Cond(swc::CondExpr {
                span: DUMMY_SP,
                test: Box::new(self.expr_to_swc(*node.condition)),
                cons: Box::new(self.block_to_swc_inlined(*node.consequent).into()),
                alt,
            })
        } else {
            let cons = self.block_to_swc_inlined(*node.consequent).into();
            swc::Expr::Cond(swc::CondExpr {
                span: DUMMY_SP,
                test: Box::new(self.expr_to_swc(*node.condition)),
                cons: Box::new(self.some(cons).into()),
                alt: Box::new(self.none().into()),
            })
        }
    }

    fn if_to_swc_extracted(&mut self, node: ast::IfExpression) -> swc::Expr {
        let is_option = node.alternate.is_none();
        let id = self.add_temp_var_to_current_block();
        self.enter_block();
        let if_stmt = self.if_to_swc_stmt(node, AssignTo::Last(id.clone()));
        self.exit_block();
        self.push_to_block(if_stmt.into());
        if is_option {
            let stmt = self.into_option(&id);
            self.push_to_block(stmt);
        }
        create_ident(&id).into()
    }

    fn if_decl_to_swc_expr(&mut self, node: ast::IfDeclExpression) -> swc::Expr {
        let is_option = node.alternate.is_none();
        let id = self.add_temp_var_to_current_block();
        self.enter_block();
        let if_stmt = self.if_decl_to_swc_stmt(node, AssignTo::Last(id.clone()));
        self.exit_block();
        self.push_to_block(if_stmt.into());
        if is_option {
            let stmt = self.into_option(&id);
            self.push_to_block(stmt);
        }
        create_ident(&id).into()
    }

    fn loop_to_swc_expr(&mut self, node: ast::Loop) -> swc::Expr {
        let id = self.add_temp_var_to_current_block();
        self.enter_block();
        let loop_stmt = self.loop_to_swc_stmt(node, AssignTo::Break(id.clone()));
        self.exit_block();
        self.push_to_block(loop_stmt.into());
        let option = self.into_option(&id);
        self.push_to_block(option);
        create_ident(&id).into()
    }

    fn match_to_swc_expr(&mut self, node: ast::MatchExpression) -> swc::Expr {
        let id = self.add_temp_var_to_current_block();
        self.enter_block();
        let stmt = self.match_to_swc_stmt(node, AssignTo::Last(id.clone()));
        self.exit_block();
        self.push_to_block(stmt.into());
        create_ident(&id).into()
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
                left: swc::PatOrExpr::Pat(Box::new(swc::Pat::Ident(swc::BindingIdent {
                    id: create_ident(to),
                    type_ann: None,
                }))),
                right: Box::new(expr),
            })),
        }
    }
}
