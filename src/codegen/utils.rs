use std::collections::HashSet;
use swc_common::DUMMY_SP;
use swc_ecma_ast::{BlockStmt, Class, ClassDecl, Constructor, Expr, Ident, Lit, Str};

use super::type_declaration::utils::{name_to_swc_param, this_assignment};

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

pub fn create_ident(name: &str) -> Ident {
    Ident {
        sym: safe_identifier(name).into(),
        span: DUMMY_SP,
        optional: false,
    }
}

pub fn create_str(text: &str) -> Expr {
    Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: text.into(),
        raw: None,
    }))
}

pub fn get_option_class() -> ClassDecl {
    let constructor = Constructor {
        span: DUMMY_SP,
        key: create_ident("constructor").into(),
        is_optional: false,
        params: vec![name_to_swc_param("__"), name_to_swc_param("some")],
        body: Some(BlockStmt {
            span: DUMMY_SP,
            stmts: vec![this_assignment("__"), this_assignment("some")],
        }),
        accessibility: None,
    };
    let class = Class {
        span: DUMMY_SP,
        body: vec![constructor.into()],
        super_class: None,
        super_type_params: None,
        decorators: vec![],
        type_params: None,
        is_abstract: false,
        implements: vec![],
    };
    ClassDecl {
        declare: false,
        ident: create_ident("__Option"),
        class: Box::new(class),
    }
}
