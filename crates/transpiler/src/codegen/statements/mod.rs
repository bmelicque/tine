mod assignments;
mod functions;
mod types;
mod utils;

use crate::codegen::{
    expressions::ExpressionResult,
    utils::{is_primitive, make_cell},
};

use super::{utils::ident_from_str, CodeGenerator};
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;
use tine_core::ir;

impl CodeGenerator<'_> {
    pub fn stmt_to_swc(&mut self, node: &ir::Statement) -> Vec<swc::Stmt> {
        match node {
            ir::Statement::Assignment(a) => self.handle_assignment(a),
            ir::Statement::Break(b) => self.handle_break(b),
            ir::Statement::Continue(_) => vec![swc::Stmt::Continue(swc::ContinueStmt::default())],
            ir::Statement::Enum(e) => vec![self.enum_def_to_swc(e).into()],
            ir::Statement::Expression(e) => match e {
                ir::Expression::Block(block) => vec![self.block_to_swc_stmt(block).into()],
                ir::Expression::If(expr) => self.if_to_swc_stmt(expr),
                ir::Expression::For(f) => self.for_to_swc_stmt(f),
                ir::Expression::ForIn(f) => self.for_in_to_swc_stmt(f),
                expr => self.handle_expression_statement(expr),
            },
            ir::Statement::Function(f) => {
                vec![self.handle_function_definition(f)]
            }
            ir::Statement::Return(node) => self.return_to_swc(node),
            ir::Statement::Struct(s) => vec![self.struct_def_to_swc(s).into()],
            ir::Statement::Use(_) => unreachable!(),
            ir::Statement::Variable(node) => self.handle_declaration(node),
        }
    }

    pub fn block_to_swc_stmt(&mut self, node: &ir::Block) -> swc::BlockStmt {
        let stmts = node
            .statements
            .iter()
            .flat_map(|stmt| self.stmt_to_swc(stmt))
            .collect::<Vec<_>>();

        swc::BlockStmt {
            stmts,
            ..Default::default()
        }
    }

    fn handle_break(&mut self, node: &ir::BreakStatement) -> Vec<swc::Stmt> {
        match &self.break_target {
            Some(target) => match &node.expression {
                Some(value) => self.handle_break_assign(target.clone(), &value),
                None => vec![swc::Stmt::Expr(swc::ExprStmt {
                    span: DUMMY_SP,
                    expr: Box::new(swc::Expr::Assign(swc::AssignExpr {
                        left: target.clone().into(),
                        right: Box::new(self.none().into()),
                        ..Default::default()
                    })),
                })],
            },
            None => vec![swc::Stmt::Break(swc::BreakStmt::default())],
        }
    }
    fn handle_break_assign(
        &mut self,
        assign_target: swc::Ident,
        value: &ir::Expression,
    ) -> Vec<swc::Stmt> {
        let value_result = self.handle_assigned_value(value);

        let mut stmts = value_result.prelim_stmts;

        let expr = swc::Expr::Assign(swc::AssignExpr {
            left: assign_target.into(),
            right: Box::new(value_result.expr),
            ..Default::default()
        });

        stmts.push(swc::Stmt::Expr(swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(expr),
        }));

        stmts
    }

    fn handle_expression_statement(&mut self, node: &ir::Expression) -> Vec<swc::Stmt> {
        let result = self.handle_expression(node);
        let mut stmts = result.prelim_stmts;
        stmts.push(swc::Stmt::Expr(swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(result.expr),
        }));
        stmts
    }

    pub fn if_to_swc_stmt(&mut self, node: &ir::IfExpression) -> Vec<swc::Stmt> {
        let test_result = self.handle_expression(&node.condition);
        let mut stmts = test_result.prelim_stmts;
        let block = self.block_to_swc_stmt(&node.consequent);
        let cons = Box::new(block.into());
        let alt = node
            .alternate
            .as_ref()
            .map(|alt| self.block_to_swc_stmt(alt).into())
            .map(Box::new);
        stmts.push(swc::Stmt::If(swc::IfStmt {
            span: DUMMY_SP,
            test: Box::new(test_result.expr),
            cons,
            alt,
        }));
        stmts
    }

    pub fn for_to_swc_stmt(&mut self, node: &ir::ForExpression) -> Vec<swc::Stmt> {
        let test_result = match node.condition.as_ref() {
            Some(condition) => self.handle_expression(condition),
            None => {
                let expr = swc::Expr::Lit(swc::Lit::Bool(swc::Bool {
                    span: DUMMY_SP,
                    value: true,
                }));
                ExpressionResult {
                    prelim_stmts: vec![],
                    expr,
                }
            }
        };
        let mut stmts = test_result.prelim_stmts;
        let body = Box::new(self.block_to_swc_stmt(&node.body).into());
        stmts.push(swc::Stmt::While(swc::WhileStmt {
            span: DUMMY_SP,
            test: Box::new(test_result.expr),
            body,
        }));
        stmts
    }

    pub fn for_in_to_swc_stmt(&mut self, node: &ir::ForInExpression) -> Vec<swc::Stmt> {
        let iterable_result = self.handle_expression(&node.iterable);
        let mut stmts = iterable_result.prelim_stmts;

        stmts.push(swc::Stmt::ForOf(swc::ForOfStmt {
            left: swc::ForHead::VarDecl(Box::new(swc::VarDecl {
                decls: vec![swc::VarDeclarator {
                    span: DUMMY_SP,
                    name: ident_from_str(&node.element.as_name()).into(),
                    init: None,
                    definite: false,
                }],
                ..Default::default()
            })),
            right: Box::new(iterable_result.expr),
            body: Box::new(self.block_to_swc_stmt(&node.body).into()),
            ..Default::default()
        }));

        stmts
    }

    fn return_to_swc(&mut self, node: &ir::ReturnStatement) -> Vec<swc::Stmt> {
        let (mut stmts, arg) = match &node.expression {
            Some(e) => {
                let result = self.handle_expression(e);
                (result.prelim_stmts, Some(result.expr))
            }
            None => (vec![], None),
        };

        stmts.push(swc::Stmt::Return(swc::ReturnStmt {
            span: DUMMY_SP,
            arg: arg.map(Box::new),
        }));

        stmts
    }

    fn handle_declaration(&mut self, node: &ir::VariableDeclaration) -> Vec<swc::Stmt> {
        let kind = if node.mutable {
            swc::VarDeclKind::Let
        } else {
            swc::VarDeclKind::Const
        };

        let expr_result = self.handle_expression(&node.value);
        let mut stmts = expr_result.prelim_stmts;

        let expr = if node.symbol.is_referenced() && is_primitive(node.symbol.as_type()) {
            make_cell(expr_result.expr)
        } else {
            expr_result.expr
        };

        stmts.push(swc::Stmt::Decl(swc::Decl::Var(Box::new(swc::VarDecl {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            kind,
            declare: false,
            decls: vec![swc::VarDeclarator {
                span: DUMMY_SP,
                name: swc::Pat::Ident(ident_from_str(&node.symbol.as_name()).into()),
                init: Some(Box::new(expr)),
                definite: false,
            }],
        }))));

        stmts
    }
}
