use crate::codegen::{utils::create_ident, CodeGenerator};
use mylang_core::ast;
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;

impl CodeGenerator<'_> {
    pub fn declaration_to_swc(&mut self, node: &ast::VariableDeclaration) -> Vec<swc::Stmt> {
        if let ast::Pattern::Identifier(_) = *node.pattern {
            return vec![self.identifier_declaration_to_swc(node).into()];
        };

        let wrappers: Vec<swc::Stmt> = node
            .pattern
            .list_identifiers()
            .into_iter()
            .filter_map(|id| self.find_symbol(id.loc()))
            .filter(|var| var.borrow().has_ref())
            .map(|var| wrap_identifier(&var.borrow().name).into())
            .collect();

        let pattern = self.pattern_to_swc(&node.pattern);
        let init = self.expr_to_swc(&node.value);
        let decl = create_declaration(swc::VarDeclKind::Let, pattern, init);

        vec![vec![decl.into()], wrappers].concat()
    }

    fn identifier_declaration_to_swc(&mut self, node: &ast::VariableDeclaration) -> swc::Decl {
        let ast::Pattern::Identifier(id) = node.pattern.as_ref() else {
            panic!()
        };

        let mut init = self.expr_to_swc(&node.value);
        let Some(info) = self.find_symbol(id.loc()) else {
            panic!(
                "expected to find symbol with name '{}' at location {:?}",
                id.as_str(),
                id.loc()
            )
        };
        if info.borrow().has_ref() {
            init = swc::Expr::Array(swc::ArrayLit {
                span: DUMMY_SP,
                elems: vec![Some(init.into())],
            })
        }

        let kind = match node.op {
            ast::DeclarationOp::Const => swc::VarDeclKind::Const,
            ast::DeclarationOp::Mut => swc::VarDeclKind::Let,
        };

        let name = swc::Pat::Ident(swc::BindingIdent {
            id: create_ident(id.as_str()),
            type_ann: None,
        });

        create_declaration(kind, name, init).into()
    }
}

fn create_declaration(kind: swc::VarDeclKind, name: swc::Pat, init: swc::Expr) -> swc::VarDecl {
    swc::VarDecl {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        kind,
        declare: false,
        decls: vec![swc::VarDeclarator {
            span: DUMMY_SP,
            name,
            init: Some(Box::new(init)),
            definite: false,
        }],
    }
}

/// Create an assignement that wraps an identifier, like this:
/// `identifier = [identifier]`
fn wrap_identifier(name: &str) -> swc::ExprStmt {
    swc::ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(swc::Expr::Assign(swc::AssignExpr {
            span: DUMMY_SP,
            op: swc::AssignOp::Assign,
            left: swc::AssignTarget::Simple(swc::SimpleAssignTarget::Ident(swc::BindingIdent {
                id: create_ident(name),
                type_ann: None,
            })),
            right: Box::new(swc::Expr::Array(swc::ArrayLit {
                span: DUMMY_SP,
                elems: vec![Some(swc::Expr::Ident(create_ident(name)).into())],
            })),
        })),
    }
}
