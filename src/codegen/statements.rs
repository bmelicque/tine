use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

use crate::ast;

use super::CodeGenerator;

impl CodeGenerator {
    pub fn stmt_to_swc(&mut self, node: ast::Statement) -> Option<swc::Stmt> {
        match node {
            ast::Statement::Assignment(node) => Some(self.assignment_to_swc(node).into()),
            ast::Statement::Empty => None,
            ast::Statement::Expression(node) => match *node.expression {
                ast::Expression::Block(block) => Some(self.block_to_swc_stmt(block, None).into()),
                ast::Expression::If(expr) => Some(self.if_to_swc_stmt(expr, None).into()),
                ast::Expression::IfDecl(expr) => Some(self.if_decl_to_swc_stmt(expr, None).into()),
                expr => Some(
                    swc::ExprStmt {
                        span: DUMMY_SP,
                        expr: Box::new(self.expr_to_swc(expr)),
                    }
                    .into(),
                ),
            },
            ast::Statement::Return(node) => Some(self.return_to_swc(node).into()),
            ast::Statement::TypeAlias(node) => self.alias_to_swc(node).into(),
            ast::Statement::VariableDeclaration(node) => Some(self.declaration_to_swc(node).into()),
        }
    }

    fn assignment_to_swc(&mut self, node: ast::Assignment) -> swc::ExprStmt {
        swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(swc::Expr::Assign(swc::AssignExpr {
                span: DUMMY_SP,
                op: swc::AssignOp::Assign,
                left: self.pat_or_expr_to_swc(node.pattern),
                right: Box::new(self.expr_to_swc(node.value)),
            })),
        }
    }

    pub fn block_to_swc_stmt(
        &mut self,
        node: ast::BlockExpression,
        assign_to: Option<&str>,
    ) -> swc::BlockStmt {
        self.push_scope();
        let mut stmts: Vec<swc::Stmt> = node
            .statements
            .iter()
            .filter_map(|stmt| self.stmt_to_swc(stmt.clone()))
            .collect();
        self.drop_scope();
        if let Some(target) = assign_to {
            if let Some(swc::Stmt::Expr(last)) = stmts.last_mut() {
                *last = self.assignment_expression(target, *last.expr.clone())
            }
        }

        swc::BlockStmt {
            span: DUMMY_SP,
            stmts,
        }
    }

    fn declaration_to_swc(&mut self, node: ast::VariableDeclaration) -> swc::Decl {
        let init = Some(Box::new(self.expr_to_swc(*node.value)));

        let decl = swc::VarDeclarator {
            span: DUMMY_SP,
            name: self.pattern_to_swc(*node.pattern),
            init,
            definite: false,
        };

        let kind = match node.op {
            ast::DeclarationOp::Mut => swc::VarDeclKind::Let,
            ast::DeclarationOp::Const => swc::VarDeclKind::Const,
        };

        swc::Decl::Var(Box::new(swc::VarDecl {
            span: DUMMY_SP,
            kind,
            declare: false,
            decls: vec![decl],
        }))
    }

    /// assign_to is Some if the last stmt has to be assigned (used for extracted blocks)
    pub fn if_to_swc_stmt(
        &mut self,
        node: ast::IfExpression,
        assign_to: Option<&str>,
    ) -> swc::IfStmt {
        let block = self.block_to_swc_stmt(*node.consequent, assign_to);
        let test = Box::new(self.expr_to_swc(*node.condition));
        let cons = Box::new(block.into());
        let alt = node
            .alternate
            .map(|alt| self.alt_to_swc_stmt(alt.as_ref(), assign_to))
            .map(Box::new);
        swc::IfStmt {
            span: DUMMY_SP,
            test,
            cons,
            alt,
        }
    }

    /// assign_to is Some if the last stmt has to be assigned (used for extracted blocks)
    pub fn if_decl_to_swc_stmt(
        &mut self,
        node: ast::IfDeclExpression,
        assign_to: Option<&str>,
    ) -> swc::IfStmt {
        let mut block = self.block_to_swc_stmt(*node.consequent, assign_to);
        let test = Box::new(self.pattern_to_swc_test(&node.pattern, &node.scrutinee));
        block.stmts.push(
            swc::Decl::Var(Box::new(swc::VarDecl {
                span: DUMMY_SP,
                kind: swc::VarDeclKind::Const,
                declare: false,
                decls: vec![swc::VarDeclarator {
                    span: DUMMY_SP,
                    name: self.pattern_to_swc(*node.pattern),
                    init: Some(Box::new(self.expr_to_swc(*node.scrutinee))),
                    definite: false,
                }],
            }))
            .into(),
        );
        block.stmts.rotate_right(1);
        let cons = Box::new(block.into());
        let alt = node
            .alternate
            .map(|alt| self.alt_to_swc_stmt(alt.as_ref(), assign_to))
            .map(Box::new);
        swc::IfStmt {
            span: DUMMY_SP,
            test,
            cons,
            alt,
        }
    }

    fn alt_to_swc_stmt(&mut self, node: &ast::Alternate, assign_to: Option<&str>) -> swc::Stmt {
        match node {
            ast::Alternate::Block(n) => self.block_to_swc_stmt(n.clone(), assign_to).into(),
            ast::Alternate::If(n) => self.if_to_swc_stmt(n.clone(), assign_to).into(),
            ast::Alternate::IfDecl(n) => self.if_decl_to_swc_stmt(n.clone(), assign_to).into(),
        }
    }

    fn return_to_swc(&mut self, node: ast::ReturnStatement) -> swc::ReturnStmt {
        swc::ReturnStmt {
            span: DUMMY_SP,
            arg: node
                .value
                .map(|value| self.expr_to_swc(*value))
                .map(Box::new),
        }
    }
}
