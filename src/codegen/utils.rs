use std::collections::HashSet;
use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

use crate::ast;

use super::{
    type_declaration::utils::{name_to_swc_param, this_assignment},
    CodeGenerator,
};

fn js_reserved_words() -> HashSet<&'static str> {
    [
        // ECMAScript Keywords
        "await",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "debugger",
        "default",
        "delete",
        "do",
        "else",
        "enum",
        "export",
        "extends",
        "false",
        "finally",
        "for",
        "function",
        "if",
        "import",
        "in",
        "instanceof",
        "new",
        "null",
        "return",
        "super",
        "switch",
        "this",
        "throw",
        "true",
        "try",
        "typeof",
        "var",
        "void",
        "while",
        "with",
        "yield",
        // Strict Mode Future Reserved Words
        "let",
        "static",
        "implements",
        "interface",
        "package",
        "private",
        "protected",
        "public",
        // Literals
        "arguments",
        "eval",
    ]
    .iter()
    .copied()
    .collect()
}

/// Takes an identifier name and returns a safe version if it's reserved.
/// If not reserved, returns the name unchanged.
fn safe_identifier(name: &str) -> String {
    let reserved = js_reserved_words();
    if reserved.contains(name) {
        format!("{name}_")
    } else {
        name.to_string()
    }
}

pub fn create_ident(name: &str) -> swc::Ident {
    swc::Ident {
        sym: safe_identifier(name).into(),
        span: DUMMY_SP,
        optional: false,
    }
}

pub fn create_str(text: &str) -> swc::Expr {
    swc::Expr::Lit(swc::Lit::Str(swc::Str {
        span: DUMMY_SP,
        value: text.into(),
        raw: None,
    }))
}

pub fn get_option_class() -> swc::ClassDecl {
    let constructor = swc::Constructor {
        span: DUMMY_SP,
        key: create_ident("constructor").into(),
        is_optional: false,
        params: vec![name_to_swc_param("__"), name_to_swc_param("some")],
        body: Some(swc::BlockStmt {
            span: DUMMY_SP,
            stmts: vec![this_assignment("__"), this_assignment("some")],
        }),
        accessibility: None,
    };
    let class = swc::Class {
        span: DUMMY_SP,
        body: vec![constructor.into()],
        super_class: None,
        super_type_params: None,
        decorators: vec![],
        type_params: None,
        is_abstract: false,
        implements: vec![],
    };
    swc::ClassDecl {
        declare: false,
        ident: create_ident("__Option"),
        class: Box::new(class),
    }
}

pub fn can_be_inlined(node: &ast::Statement) -> bool {
    match node {
        ast::Statement::Expression(e) => match e.expression.as_ref() {
            ast::Expression::Block(b) => b.can_be_inlined(),
            ast::Expression::If(i) => i.can_be_inlined(),
            ast::Expression::IfDecl(_) => false,
            // TODO: loops
            // TODO: match statements
            _ => true,
        },
        _ => false,
    }
}

impl ast::BlockExpression {
    pub fn can_be_inlined(&self) -> bool {
        self.statements
            .iter()
            .find(|st| !can_be_inlined(st))
            .is_none()
    }
}

impl ast::IfExpression {
    pub fn can_be_inlined(&self) -> bool {
        if !self.consequent.can_be_inlined() {
            return false;
        }
        let Some(ref alt) = self.alternate else {
            return true;
        };
        match alt.as_ref() {
            ast::Alternate::Block(b) => b.can_be_inlined(),
            ast::Alternate::If(i) => i.can_be_inlined(),
            ast::Alternate::IfDecl(_) => false,
        }
    }
}

pub fn undefined() -> swc::Expr {
    swc::Expr::Ident(swc::Ident {
        span: DUMMY_SP,
        sym: "undefined".into(),
        optional: false,
    })
}

pub fn true_lit() -> swc::Expr {
    swc::Expr::Lit(swc::Lit::Bool(swc::Bool {
        span: DUMMY_SP,
        value: true,
    }))
}

impl CodeGenerator {
    pub fn into_option(&mut self, identifier: &String) -> swc::Stmt {
        // identifier !== undefined ? new __Option("Some", identifier) : new __Option("None")

        let test = Box::new(swc::Expr::Bin(swc::BinExpr {
            span: DUMMY_SP,
            op: swc::BinaryOp::NotEqEq,
            left: Box::new(create_ident(&identifier).into()),
            right: Box::new(undefined()),
        }));

        let cons = Box::new(self.some(create_ident(&identifier).into()).into());
        let alt = Box::new(self.none().into());
        let expr = Box::new(swc::Expr::Cond(swc::CondExpr {
            span: DUMMY_SP,
            test,
            cons,
            alt,
        }));

        swc::Stmt::Expr(swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(swc::Expr::Assign(swc::AssignExpr {
                span: DUMMY_SP,
                op: swc::AssignOp::Assign,
                left: swc::PatOrExpr::Pat(Box::new(swc::Pat::Ident(swc::BindingIdent {
                    id: create_ident(&identifier),
                    type_ann: None,
                }))),
                right: expr,
            })),
        })
    }
}
