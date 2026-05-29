use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

use crate::codegen::{expressions::ExpressionResult, CodeGenerator};

impl CodeGenerator<'_> {
    pub(super) fn extract_expression(&mut self, expr: swc::Expr) -> ExpressionResult {
        let temp = self.get_temp_id();
        let prelim = swc::Stmt::Decl(swc::Decl::Var(Box::new(swc::VarDecl {
            kind: swc::VarDeclKind::Const,
            decls: vec![swc::VarDeclarator {
                span: DUMMY_SP,
                name: swc::Pat::Ident(temp.clone().into()),
                init: Some(Box::new(expr)),
                definite: false,
            }],
            ..Default::default()
        })));
        ExpressionResult {
            prelim_stmts: vec![prelim],
            expr: temp.into(),
        }
    }

    pub fn to_extracted(&mut self, result: ExpressionResult) -> ExpressionResult {
        if result.prelim_stmts.is_empty() && !matches!(result.expr, swc::Expr::Lit(_)) {
            self.extract_expression(result.expr)
        } else {
            result
        }
    }

    pub fn extract_all(
        &mut self,
        results: Vec<ExpressionResult>,
    ) -> (Vec<swc::Stmt>, Vec<swc::Expr>) {
        let needs_extract = results
            .iter()
            .find(|r| !r.prelim_stmts.is_empty())
            .is_some();
        if !needs_extract {
            let exprs = results.into_iter().map(|r| r.expr).collect();
            return (vec![], exprs);
        }
        results.into_iter().map(|r| self.to_extracted(r)).fold(
            (Vec::new(), Vec::new()),
            |mut acc, r| {
                acc.0.extend(r.prelim_stmts);
                acc.1.push(r.expr);
                acc
            },
        )
    }

    /// Extract elements until meeting the last one already extracted.
    /// If nothing was already extraced, nothing more will be.
    pub fn extract_necessary(
        &mut self,
        results: Vec<ExpressionResult>,
    ) -> (Vec<swc::Stmt>, Vec<swc::Expr>) {
        let last = results
            .iter()
            .enumerate()
            .rev()
            .find(|(_, r)| !r.prelim_stmts.is_empty())
            .map(|(i, _)| i);
        let Some(last) = last else {
            let exprs = results.into_iter().map(|r| r.expr).collect();
            return (vec![], exprs);
        };
        results
            .into_iter()
            .enumerate()
            .map(|(i, r)| if i <= last { self.to_extracted(r) } else { r })
            .fold((Vec::new(), Vec::new()), |mut acc, r| {
                acc.0.extend(r.prelim_stmts);
                acc.1.push(r.expr);
                acc
            })
    }
}

pub fn ident_to_declaration(ident: swc::Ident) -> swc::Stmt {
    swc::Stmt::Decl(swc::Decl::Var(Box::new(swc::VarDecl {
        kind: swc::VarDeclKind::Let,
        decls: vec![swc::VarDeclarator {
            span: DUMMY_SP,
            name: swc::Pat::Ident(ident.into()),
            init: None,
            definite: false,
        }],
        ..Default::default()
    })))
}

/// Transform the last statement of the block so it can handle assignment to an outer value.
///
/// For example:
/// ```js
/// {
///     //...
///     doStuff()
/// }
/// ```
///
/// becomes:
/// ```js
/// {
///     //...
///     temp = doStuff()
/// }
/// ```
///
/// This function also handles nested blocks and `if` statements.
pub fn assign_block_last_expression(block: &mut swc::BlockStmt, target: swc::Ident) {
    match block.stmts.last_mut() {
        Some(stmt) => {
            assign_helper(stmt, target);
        }
        None => {}
    }
}

pub fn assign_if_last_expressions(node: &mut swc::IfStmt, target: swc::Ident) {
    assign_helper(&mut node.cons, target.clone());
    match &mut node.alt {
        Some(alt) => assign_helper(&mut *alt, target),
        None => {}
    }
}

fn assign_helper(mut stmt: &mut swc::Stmt, target: swc::Ident) {
    match &mut stmt {
        swc::Stmt::Block(ref mut b) => {
            assign_block_last_expression(b, target);
        }
        swc::Stmt::If(ref mut i) => {
            assign_if_last_expressions(i, target);
        }
        swc::Stmt::Expr(e) => {
            let assignment = swc::AssignExpr {
                left: swc::AssignTarget::Simple(swc::SimpleAssignTarget::Ident(target.into())),
                right: e.expr.clone(),
                ..Default::default()
            };
            *stmt = swc::Stmt::Expr(swc::ExprStmt {
                span: DUMMY_SP,
                expr: Box::new(assignment.into()),
            });
        }
        _ => {}
    }
}
