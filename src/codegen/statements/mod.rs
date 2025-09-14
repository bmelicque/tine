mod assignments;

use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

use crate::{ast, codegen::utils::AssignTo};

use super::{utils::create_ident, CodeGenerator};

impl CodeGenerator {
    pub fn stmt_to_swc(&mut self, node: ast::Statement) -> Vec<swc::Stmt> {
        match node {
            ast::Statement::Assignment(node) => vec![self.assignment_to_swc(node).into()],
            ast::Statement::Break(_) => vec![self.break_to_swc_stmt().into()],
            ast::Statement::Empty => vec![],
            ast::Statement::Expression(node) => match *node.expression {
                ast::Expression::Block(block) => {
                    vec![self.block_to_swc_stmt(block, AssignTo::None).into()]
                }
                ast::Expression::If(expr) => vec![self.if_to_swc_stmt(expr, AssignTo::None).into()],
                ast::Expression::IfDecl(expr) => {
                    vec![self.if_decl_to_swc_stmt(expr, AssignTo::None).into()]
                }
                ast::Expression::Loop(expr) => vec![self.loop_to_swc_stmt(expr, AssignTo::None)],
                ast::Expression::Match(expr) => {
                    vec![self.match_to_swc_stmt(expr, AssignTo::None).into()]
                }
                expr => vec![swc::ExprStmt {
                    span: DUMMY_SP,
                    expr: Box::new(self.expr_to_swc(expr)),
                }
                .into()],
            },
            ast::Statement::MethodDefinition(node) => vec![swc::ExprStmt {
                span: DUMMY_SP,
                expr: Box::new(self.method_definition_to_swc(node).into()),
            }
            .into()],
            ast::Statement::Return(node) => vec![self.return_to_swc(node).into()],
            ast::Statement::TypeAlias(node) => self.alias_to_swc(node).into(),
            ast::Statement::VariableDeclaration(node) => vec![self.declaration_to_swc(node).into()],
        }
    }

    pub fn block_to_swc_stmt(
        &mut self,
        node: ast::BlockExpression,
        assign_to: AssignTo,
    ) -> swc::BlockStmt {
        self.push_scope();
        let mut stmts = Vec::<swc::Stmt>::new();
        for statement in node.statements.iter() {
            match statement {
                ast::Statement::Break(stmt) => {
                    if let (AssignTo::Break(assignee), Some(expr)) =
                        (assign_to.clone(), stmt.value.clone())
                    {
                        let assigned = self.expr_to_swc(*expr.clone());
                        stmts.push(self.assignment_expression(&assignee, assigned).into());
                    }
                    stmts.push(swc::Stmt::Break(swc::BreakStmt {
                        span: DUMMY_SP,
                        label: None,
                    }));
                }
                stmt => {
                    stmts.extend(self.stmt_to_swc(stmt.clone()));
                }
            }
        }

        self.drop_scope();
        if let AssignTo::Last(ref target) = assign_to {
            if let Some(swc::Stmt::Expr(last)) = stmts.last_mut() {
                *last = self.assignment_expression(target, *last.expr.clone())
            }
        }

        swc::BlockStmt {
            span: DUMMY_SP,
            stmts,
        }
    }

    fn break_to_swc_stmt(&mut self) -> swc::BreakStmt {
        swc::BreakStmt {
            span: DUMMY_SP,
            label: None,
        }
    }

    fn declaration_to_swc(&mut self, node: ast::VariableDeclaration) -> swc::Decl {
        let init = Some(Box::new(self.expr_to_swc(*node.value)));

        let _identifiers = node.pattern.list_identifiers();
        let pattern = self.pattern_to_swc(*node.pattern);
        let decl = swc::VarDeclarator {
            span: DUMMY_SP,
            name: pattern,
            init,
            definite: false,
        };

        // TODO: wrap bindings to build pointers if needed

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
    pub fn if_to_swc_stmt(&mut self, node: ast::IfExpression, assign_to: AssignTo) -> swc::IfStmt {
        let block = self.block_to_swc_stmt(*node.consequent, assign_to.clone());
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
        assign_to: AssignTo,
    ) -> swc::IfStmt {
        let mut block = self.block_to_swc_stmt(*node.consequent, assign_to.clone());
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

    fn alt_to_swc_stmt(&mut self, node: &ast::Alternate, params: AssignTo) -> swc::Stmt {
        match node {
            ast::Alternate::Block(n) => self.block_to_swc_stmt(n.clone(), params).into(),
            ast::Alternate::If(n) => self.if_to_swc_stmt(n.clone(), params).into(),
            ast::Alternate::IfDecl(n) => self.if_decl_to_swc_stmt(n.clone(), params).into(),
        }
    }

    pub fn loop_to_swc_stmt(&mut self, node: ast::Loop, assign_to: AssignTo) -> swc::Stmt {
        match node {
            ast::Loop::For(node) => self.for_to_swc_stmt(node, assign_to).into(),
            ast::Loop::ForIn(node) => self.for_in_to_swc_stmt(node, assign_to).into(),
        }
    }

    fn for_to_swc_stmt(&mut self, node: ast::ForExpression, assign_to: AssignTo) -> swc::WhileStmt {
        let test = Box::new(self.expr_to_swc(*node.condition));
        // FIXME: no assign_to! and breaks should be `assignee = value; break;`
        let body = Box::new(self.block_to_swc_stmt(node.body, assign_to).into());
        swc::WhileStmt {
            span: DUMMY_SP,
            test,
            body,
        }
    }

    fn for_in_to_swc_stmt(
        &mut self,
        node: ast::ForInExpression,
        assign_to: AssignTo,
    ) -> swc::ForOfStmt {
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
            body: Box::new(self.get_for_in_body(&node, assign_to).into()),
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

    fn get_for_in_body(
        &mut self,
        node: &ast::ForInExpression,
        assign_to: AssignTo,
    ) -> swc::BlockStmt {
        let mut body = self.block_to_swc_stmt(node.body.clone(), assign_to);
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

    pub fn match_to_swc_stmt(
        &mut self,
        mut node: ast::MatchExpression,
        assign_to: AssignTo,
    ) -> swc::IfStmt {
        // FIXME: extract scrutinee in case expensive or not idempotent
        let mut stmt =
            self.arm_to_swc_if_stmt(node.arms.pop().unwrap(), node.scrutinee.clone(), None);

        for arm in node.arms.into_iter().rev() {
            stmt = self.arm_to_swc_if_stmt(arm, node.scrutinee.clone(), Some(Box::new(stmt)));
        }

        self.if_decl_to_swc_stmt(stmt, assign_to)
    }

    fn arm_to_swc_if_stmt(
        &mut self,
        node: ast::MatchArm,
        scrutinee: Box<ast::Expression>,
        alternate: Option<Box<ast::IfDeclExpression>>,
    ) -> ast::IfDeclExpression {
        let consequent = Box::new(ast::BlockExpression {
            span: node.expression.as_span(),
            statements: vec![ast::Statement::Expression(ast::ExpressionStatement {
                expression: node.expression.into(),
            })],
        });
        ast::IfDeclExpression {
            span: node.span,
            pattern: node.pattern,
            scrutinee,
            consequent,
            alternate: alternate
                .map(|alt| ast::Alternate::IfDecl(*alt))
                .map(Box::new),
        }
    }

    fn function_to_swc_function(&mut self, node: ast::FunctionExpression) -> swc::Function {
        let params = node
            .params
            .into_iter()
            .map(|param| swc::Param {
                span: DUMMY_SP,
                decorators: vec![],
                pat: swc::Pat::Ident(create_ident(param.name.as_str()).into()),
            })
            .collect();

        let body = match self.function_body_to_swc(node.body) {
            swc::BlockStmtOrExpr::BlockStmt(block) => block,
            swc::BlockStmtOrExpr::Expr(expr) => swc::BlockStmt {
                span: DUMMY_SP,
                stmts: vec![swc::Stmt::Return(swc::ReturnStmt {
                    span: DUMMY_SP,
                    arg: Some(expr),
                })],
            },
        };

        swc::Function {
            span: DUMMY_SP,
            params,
            decorators: vec![],
            body: Some(body),
            is_generator: false,
            is_async: false,
            type_params: None,
            return_type: None,
        }
    }

    fn method_definition_to_swc(&mut self, node: ast::MethodDefinition) -> swc::AssignExpr {
        swc::AssignExpr {
            span: DUMMY_SP,
            op: swc::AssignOp::Assign,
            left: swc::PatOrExpr::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(create_ident(&node.receiver.ty.name).into()),
                prop: swc::MemberProp::Ident(create_ident(node.name.as_str())),
            }))),
            right: Box::new(self.function_to_swc_function(node.definition).into()),
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
