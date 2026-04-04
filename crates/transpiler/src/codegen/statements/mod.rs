mod assignments;

use super::{utils::create_ident, CodeGenerator};
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;
use tine_core::ir;

impl CodeGenerator<'_> {
    pub fn stmt_to_swc(&mut self, node: &ir::Statement) -> swc::Stmt {
        match node {
            ir::Statement::Assignment(a) => self.assignment_to_swc(a).into(),
            ir::Statement::Break(_) => self.break_to_swc_stmt().into(),
            ir::Statement::Continue(_) => swc::Stmt::Continue(swc::ContinueStmt {
                span: DUMMY_SP,
                label: None,
            }),
            ir::Statement::Expression(e) => match e {
                ir::Expression::Block(block) => self.block_to_swc_stmt(block).into(),
                ir::Expression::If(expr) => self.if_to_swc_stmt(expr).into(),
                ir::Expression::For(f) => self.for_to_swc_stmt(f).into(),
                ir::Expression::ForIn(f) => self.for_in_to_swc_stmt(f).into(),
                expr => swc::Stmt::Expr(swc::ExprStmt {
                    span: DUMMY_SP,
                    expr: Box::new(self.expr_to_swc(expr)),
                }),
            },
            ir::Statement::Function(f) => {
                swc::Stmt::Decl(swc::Decl::Fn(self.function_to_swc_definition(f)))
            }
            ir::Statement::Return(node) => self.return_to_swc(node).into(),
            ir::Statement::Use(_) => unreachable!(),
            ir::Statement::Variable(node) => self.declaration_to_swc(node).into(),
        }
    }

    pub fn block_to_swc_stmt(&mut self, node: &ir::Block) -> swc::BlockStmt {
        swc::BlockStmt {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            stmts: node
                .statements
                .iter()
                .map(|stmt| self.stmt_to_swc(stmt))
                .collect(),
        }
    }

    fn break_to_swc_stmt(&mut self) -> swc::BreakStmt {
        swc::BreakStmt {
            span: DUMMY_SP,
            label: None,
        }
    }

    pub fn if_to_swc_stmt(&mut self, node: &ir::IfExpression) -> swc::IfStmt {
        let block = self.block_to_swc_stmt(&node.consequent);
        let test = Box::new(self.expr_to_swc(&node.condition));
        let cons = Box::new(block.into());
        let alt = node
            .alternate
            .as_ref()
            .map(|alt| self.block_to_swc_stmt(alt).into())
            .map(Box::new);
        swc::IfStmt {
            span: DUMMY_SP,
            test,
            cons,
            alt,
        }
    }

    pub fn for_to_swc_stmt(&mut self, node: &ir::ForExpression) -> swc::WhileStmt {
        let test = match node.condition.as_ref() {
            Some(condition) => self.expr_to_swc(condition),
            None => swc::Expr::Lit(swc::Lit::Bool(swc::Bool {
                span: DUMMY_SP,
                value: true,
            })),
        };
        let body = Box::new(self.block_to_swc_stmt(&node.body).into());
        swc::WhileStmt {
            span: DUMMY_SP,
            test: Box::new(test),
            body,
        }
    }

    pub fn for_in_to_swc_stmt(&mut self, node: &ir::ForInExpression) -> swc::ForOfStmt {
        swc::ForOfStmt {
            span: DUMMY_SP,
            is_await: false,
            left: swc::ForHead::VarDecl(Box::new(swc::VarDecl {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                kind: swc::VarDeclKind::Const,
                declare: false,
                decls: vec![swc::VarDeclarator {
                    span: DUMMY_SP,
                    name: create_ident(&node.element.as_name()).into(),
                    init: None,
                    definite: false,
                }],
            })),
            right: Box::new(self.expr_to_swc(&node.iterable)),
            body: Box::new(self.block_to_swc_stmt(&node.body).into()),
        }
    }

    fn function_to_swc_definition(&mut self, node: &ir::FunctionDefinition) -> swc::FnDecl {
        swc::FnDecl {
            ident: create_ident(&node.name.as_name()),
            declare: false,
            function: Box::new(swc::Function {
                params: node
                    .params
                    .iter()
                    .map(|p| swc::Param {
                        span: DUMMY_SP,
                        decorators: vec![],
                        pat: swc::Pat::Ident(create_ident(&p.as_name()).into()),
                    })
                    .collect(),
                decorators: vec![],
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                body: Some(self.block_to_swc_stmt(&node.body)),
                is_generator: false,
                is_async: false,
                type_params: None,
                return_type: None,
            }),
        }
    }

    fn return_to_swc(&mut self, node: &ir::ReturnStatement) -> swc::ReturnStmt {
        swc::ReturnStmt {
            span: DUMMY_SP,
            arg: node
                .expression
                .as_ref()
                .map(|value| self.expr_to_swc(&value))
                .map(Box::new),
        }
    }

    fn declaration_to_swc(&mut self, node: &ir::VariableDeclaration) -> swc::VarDecl {
        let kind = if node.mutable {
            swc::VarDeclKind::Let
        } else {
            swc::VarDeclKind::Const
        };

        swc::VarDecl {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            kind,
            declare: false,
            decls: vec![swc::VarDeclarator {
                span: DUMMY_SP,
                name: swc::Pat::Ident(create_ident(&node.symbol.as_name()).into()),
                init: Some(Box::new(self.expr_to_swc(&node.value))),
                definite: false,
            }],
        }
    }
}
