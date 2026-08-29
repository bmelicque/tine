mod assignments;
mod functions;
mod matches;
pub mod types;
mod utils;

use crate::{codegen::utils::create_bool, utils::is_declaration_mutable};

use super::{utils::ident_from_str, CodeGenerator};
use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use tine_ir as ir;

impl CodeGenerator<'_, '_> {
    pub fn stmt_to_swc(&mut self, node: ir::Statement) -> Vec<swc::Stmt> {
        match node {
            ir::Statement::Assignment(a) => self.handle_assignment(a),
            ir::Statement::Break(b) => self.handle_break(b),
            ir::Statement::Continue(_) => vec![swc::Stmt::Continue(swc::ContinueStmt::default())],
            ir::Statement::Enum(e) => vec![self.enum_def_to_swc(e).into()],
            ir::Statement::Expression(e) => match e {
                ir::Expression::Block(block) => vec![self.handle_block_stmt(block).into()],
                ir::Expression::If(expr) => self.if_to_swc_stmt(expr),
                ir::Expression::For(f) => self.for_to_swc_stmt(f),
                ir::Expression::ForIn(f) => self.for_in_to_swc_stmt(f),
                ir::Expression::Match(m) => self.handle_match_statement(m),
                expr => self.handle_expression_statement(expr),
            },
            ir::Statement::Function(f) => {
                vec![self.handle_function_definition(f)]
            }
            ir::Statement::Method(m) => {
                vec![self.handle_method_definition(m)]
            }
            ir::Statement::Return(node) => self.return_to_swc(node),
            ir::Statement::Struct(s) => vec![self.struct_def_to_swc(s).into()],
            ir::Statement::Use(_) => unreachable!(),
            ir::Statement::Variable(node) => self.handle_declaration(node),
        }
    }

    pub fn handle_block_stmt(&mut self, node: ir::Block) -> swc::BlockStmt {
        let stmts = node
            .statements
            .into_iter()
            .flat_map(|stmt| self.stmt_to_swc(stmt))
            .collect::<Vec<_>>();

        swc::BlockStmt {
            stmts,
            ..Default::default()
        }
    }

    fn handle_break(&mut self, node: ir::BreakStatement) -> Vec<swc::Stmt> {
        match &self.break_target {
            Some(target) => match node.expression {
                Some(value) => self.handle_break_assign(target.clone(), *value),
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
        value: ir::Expression,
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

    fn handle_expression_statement(&mut self, node: ir::Expression) -> Vec<swc::Stmt> {
        let result = self.handle_expression(node);
        let mut stmts = result.prelim_stmts;
        stmts.push(swc::Stmt::Expr(swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(result.expr),
        }));
        stmts
    }

    pub fn if_to_swc_stmt(&mut self, node: ir::IfExpression) -> Vec<swc::Stmt> {
        let test_result = self.handle_expression(*node.condition);
        let mut stmts = test_result.prelim_stmts;
        let block = self.handle_block_stmt(node.consequent);
        let cons = Box::new(block.into());
        let alt = node
            .alternate
            .map(|alt| self.handle_block_stmt(alt).into())
            .map(Box::new);
        stmts.push(swc::Stmt::If(swc::IfStmt {
            span: DUMMY_SP,
            test: Box::new(test_result.expr),
            cons,
            alt,
        }));
        stmts
    }

    pub fn for_to_swc_stmt(&mut self, node: ir::ForExpression) -> Vec<swc::Stmt> {
        let test_result = match node.condition {
            Some(condition) => self.handle_expression(*condition),
            None => create_bool(true).into(),
        };
        let mut stmts = test_result.prelim_stmts;
        let body = Box::new(self.handle_block_stmt(node.body).into());
        stmts.push(swc::Stmt::While(swc::WhileStmt {
            span: DUMMY_SP,
            test: Box::new(test_result.expr),
            body,
        }));
        stmts
    }

    pub fn for_in_to_swc_stmt(&mut self, node: ir::ForInExpression) -> Vec<swc::Stmt> {
        let iterable_result = self.handle_expression(*node.iterable);
        let mut stmts = iterable_result.prelim_stmts;

        let name = self.symbol_name(node.element.symbol);
        stmts.push(swc::Stmt::ForOf(swc::ForOfStmt {
            left: swc::ForHead::VarDecl(Box::new(swc::VarDecl {
                decls: vec![swc::VarDeclarator {
                    span: DUMMY_SP,
                    name: ident_from_str(name).into(),
                    init: None,
                    definite: false,
                }],
                ..Default::default()
            })),
            right: Box::new(iterable_result.expr),
            body: Box::new(self.handle_block_stmt(node.body).into()),
            ..Default::default()
        }));

        stmts
    }

    fn return_to_swc(&mut self, node: ir::ReturnStatement) -> Vec<swc::Stmt> {
        let (mut stmts, arg) = match node.expression {
            Some(e) => {
                let result = self.handle_expression(*e);
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

    fn handle_declaration(&mut self, node: ir::VariableDeclaration) -> Vec<swc::Stmt> {
        let (mut stmts, decl) = self.declaration_helper(node);
        stmts.push(swc::Stmt::Decl(decl));

        stmts
    }

    pub fn declaration_helper(
        &mut self,
        node: ir::VariableDeclaration,
    ) -> (Vec<swc::Stmt>, swc::Decl) {
        let mutable = is_declaration_mutable(&node, self.symbols);
        let kind = if mutable {
            swc::VarDeclKind::Let
        } else {
            swc::VarDeclKind::Const
        };

        let expr_result = self.handle_expression(node.value);
        let stmts = expr_result.prelim_stmts;
        let pat_result = self.handle_pattern(node.pattern, swc::Expr::default());
        debug_assert!(pat_result.test.is_none());
        let decl = swc::Decl::Var(Box::new(swc::VarDecl {
            kind,
            decls: vec![swc::VarDeclarator {
                span: DUMMY_SP,
                name: pat_result.decl.unwrap(),
                init: Some(Box::new(expr_result.expr)),
                definite: false,
            }],
            ..Default::default()
        }));

        (stmts, decl)
    }
}
