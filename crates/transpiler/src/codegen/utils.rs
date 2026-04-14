use std::collections::HashSet;
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;

use tine_core::{ir, types::TypeId, TypeStore};

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

pub fn create_block_stmt(stmts: Vec<swc::Stmt>) -> swc::BlockStmt {
    swc::BlockStmt {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        stmts,
    }
}

pub fn can_be_inlined(node: &ir::Statement) -> bool {
    match node {
        ir::Statement::Assignment(a) => can_expression_be_inlined(&a.value),
        ir::Statement::Expression(e) => can_expression_be_inlined(e),
        _ => false,
    }
}

pub fn can_expression_be_inlined(node: &ir::Expression) -> bool {
    match node {
        ir::Expression::Block(b) => can_block_be_inlined(b),
        ir::Expression::If(i) => can_ifexpr_be_inlined(i),
        ir::Expression::For(_) => false,
        ir::Expression::ForIn(_) => false,
        _ => true,
    }
}

pub fn can_block_be_inlined(block: &ir::Block) -> bool {
    block
        .statements
        .iter()
        .find(|st| !can_be_inlined(st))
        .is_none()
}

pub fn can_ifexpr_be_inlined(expr: &ir::IfExpression) -> bool {
    if !can_block_be_inlined(&expr.consequent) {
        return false;
    }
    match &expr.alternate {
        Some(alt) => can_block_be_inlined(alt),
        None => true,
    }
}

pub fn is_primitive(ty: TypeId) -> bool {
    match ty {
        TypeStore::BOOLEAN
        | TypeStore::FLOAT
        | TypeStore::INTEGER
        | TypeStore::STRING
        | TypeStore::UNIT => true,
        _ => false,
    }
}

pub fn is_handled_by_ref(node: &ir::Expression) -> bool {
    if !is_primitive(node.ty()) {
        return true;
    }

    match node {
        ir::Expression::Identifier(i) => i.symbol.is_referenced(),
        _ => false,
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

pub fn make_cell(value: swc::Expr) -> swc::Expr {
    let callee = swc::Expr::Member(swc::MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(create_ident("$").into()),
        prop: swc::MemberProp::Ident(create_ident("Cell").into()),
    });

    swc::Expr::New(swc::NewExpr {
        callee: Box::new(callee),
        args: Some(vec![value.into()]),
        ..Default::default()
    })
}

impl CodeGenerator<'_> {
    pub fn none(&mut self) -> swc::NewExpr {
        let args = vec![swc::ExprOrSpread {
            spread: None,
            expr: Box::new(create_str("None")),
        }];

        swc::NewExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(swc::Expr::Ident(create_ident("$"))),
                prop: swc::MemberProp::Ident(create_ident("Option").into()),
            })),
            args: Some(args),
            type_args: None,
        }
    }
}
