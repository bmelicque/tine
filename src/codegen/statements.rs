use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

use crate::ast;

use super::{utils::create_ident, CodeGenerator};

impl CodeGenerator {
    pub fn stmt_to_swc(&mut self, node: ast::Statement) -> Option<swc::Stmt> {
        match node {
            ast::Statement::Assignment(node) => Some(self.assignment_to_swc(node).into()),
            ast::Statement::Empty => None,
            ast::Statement::Expression(node) => match *node.expression {
                ast::Expression::Block(block) => Some(self.block_to_swc_stmt(block, None).into()),
                ast::Expression::If(expr) => Some(self.if_to_swc_stmt(expr, None).into()),
                ast::Expression::IfDecl(expr) => Some(self.if_decl_to_swc_stmt(expr, None).into()),
                ast::Expression::Loop(expr) => Some(self.loop_to_swc_stmt(expr)),
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

    pub fn loop_to_swc_stmt(&mut self, node: ast::Loop) -> swc::Stmt {
        match node {
            ast::Loop::For(node) => self.for_to_swc_stmt(node).into(),
            ast::Loop::ForIn(node) => self.for_in_to_swc_stmt(node).into(),
        }
    }

    fn for_to_swc_stmt(&mut self, node: ast::ForExpression) -> swc::WhileStmt {
        let test = Box::new(self.expr_to_swc(*node.condition));
        // FIXME: no assign_to! and breaks should be `assignee = value; break;`
        let body = Box::new(self.block_to_swc_stmt(node.body, None).into());
        swc::WhileStmt {
            span: DUMMY_SP,
            test,
            body,
        }
    }

    fn for_in_to_swc_stmt(&mut self, node: ast::ForInExpression) -> swc::ForOfStmt {
        swc::ForOfStmt {
            span: DUMMY_SP,
            is_await: false,
            left: swc::ForHead::VarDecl(Box::new(swc::VarDecl {
                span: DUMMY_SP,
                kind: swc::VarDeclKind::Const,
                declare: false,
                decls: vec![swc::VarDeclarator {
                    span: DUMMY_SP,
                    name: self.get_for_in_element_name(node.pattern.as_ref()),
                    init: None,
                    definite: false,
                }],
            })),
            right: Box::new(self.expr_to_swc(*node.iterable.clone())),
            body: Box::new(self.get_for_in_body(&node).into()),
        }
    }

    fn get_for_in_element_name(&mut self, pattern: &ast::Pattern) -> swc::Pat {
        match pattern {
            ast::Pattern::Identifier(id) => self.identifier_pattern_to_swc(id.clone()),
            _ => swc::Pat::Ident(swc::BindingIdent {
                id: create_ident("__"),
                type_ann: None,
            }),
        }
    }

    fn get_for_in_body(&mut self, node: &ast::ForInExpression) -> swc::BlockStmt {
        let mut body = self.block_to_swc_stmt(node.body.clone(), None);
        if matches!(*node.pattern, ast::Pattern::Identifier(_)) {
            return body;
        }
        let guard = swc::Stmt::If(swc::IfStmt {
            span: DUMMY_SP,
            test: Box::new(swc::Expr::Unary(swc::UnaryExpr {
                span: DUMMY_SP,
                op: swc::UnaryOp::Bang,
                arg: Box::new(self.pattern_to_swc_test(&node.pattern, &node.iterable)),
            })),
            cons: Box::new(swc::Stmt::Continue(swc::ContinueStmt {
                span: DUMMY_SP,
                label: None,
            })),
            alt: None,
        });
        let decl = swc::Stmt::Decl(swc::Decl::Var(Box::new(swc::VarDecl {
            span: DUMMY_SP,
            kind: swc::VarDeclKind::Const,
            declare: false,
            decls: vec![swc::VarDeclarator {
                span: DUMMY_SP,
                name: self.pattern_to_swc(*node.pattern.clone()),
                init: Some(Box::new(create_ident("__").into())),
                definite: false,
            }],
        })));
        body.stmts.push(guard);
        body.stmts.push(decl);
        body.stmts.rotate_right(2);
        body
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
