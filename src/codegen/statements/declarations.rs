use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;

use crate::{
    ast,
    codegen::{utils::create_ident, CodeGenerator},
};

impl CodeGenerator {
    pub fn declaration_to_swc(&mut self, node: ast::VariableDeclaration) -> Vec<swc::Stmt> {
        if let ast::Pattern::Identifier(_) = *node.pattern {
            return vec![self.identifier_declaration_to_swc(node).into()];
        };

        let wrappers: Vec<swc::Stmt> = node
            .pattern
            .list_identifiers()
            .into_iter()
            .filter_map(|id| self.get_info(id.span.as_str()))
            .filter(|symbol| symbol.has_ref())
            .map(|symbol| wrap_identifier(&symbol.name).into())
            .collect();

        let pattern = self.pattern_to_swc(*node.pattern);
        let init = self.expr_to_swc(*node.value);
        let decl = create_declaration(swc::VarDeclKind::Let, pattern, init);

        vec![vec![decl.into()], wrappers].concat()
    }

    fn identifier_declaration_to_swc(&mut self, node: ast::VariableDeclaration) -> swc::Decl {
        let ast::Pattern::Identifier(id) = *node.pattern else {
            panic!()
        };

        let mut init = self.expr_to_swc(*node.value);
        let info = self.get_info(id.span.as_str()).unwrap();
        if info.has_ref() {
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
            id: create_ident(id.span.as_str()),
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
