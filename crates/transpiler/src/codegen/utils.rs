use std::collections::HashSet;
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;

use mylang_core::ast;

use super::CodeGenerator;

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
        ctxt: SyntaxContext::empty(),
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

pub fn create_number(value: f64) -> swc::Expr {
    swc::Expr::Lit(swc::Lit::Num(swc::Number {
        span: DUMMY_SP,
        value: value,
        raw: None,
    }))
}

pub fn create_block_stmt(stmts: Vec<swc::Stmt>) -> swc::BlockStmt {
    swc::BlockStmt {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        stmts,
    }
}

pub fn can_be_inlined(node: &ast::Statement) -> bool {
    match node {
        ast::Statement::Expression(e) => match e.expression.as_ref() {
            ast::Expression::Block(b) => can_block_be_inlined(b),
            ast::Expression::If(i) => can_ifexpr_be_inlined(i),
            ast::Expression::IfDecl(_) => false,
            ast::Expression::Loop(_) => false,
            ast::Expression::Match(_) => false,
            _ => true,
        },
        _ => false,
    }
}

pub fn can_block_be_inlined(block: &ast::BlockExpression) -> bool {
    block
        .statements
        .iter()
        .find(|st| !can_be_inlined(st))
        .is_none()
}

pub fn can_ifexpr_be_inlined(expr: &ast::IfExpression) -> bool {
    if can_block_be_inlined(&expr.consequent) {
        return false;
    }
    let Some(ref alt) = expr.alternate else {
        return true;
    };
    match alt.as_ref() {
        ast::Alternate::Block(b) => can_block_be_inlined(b),
        ast::Alternate::If(i) => can_ifexpr_be_inlined(i),
        ast::Alternate::IfDecl(_) => false,
    }
}

pub fn undefined() -> swc::Expr {
    swc::Expr::Ident(swc::Ident {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
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
                left: swc::AssignTarget::Simple(swc::SimpleAssignTarget::Ident(
                    swc::BindingIdent {
                        id: create_ident(&identifier),
                        type_ann: None,
                    },
                )),
                right: expr,
            })),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignTo {
    None,
    /// `{ value }` becomes `{ identifier = value }`
    Last(String),
    /// `{ break value }` becomes `{ identifier = value; break }`
    Break(String),
}
