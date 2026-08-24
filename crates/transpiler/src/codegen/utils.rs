use std::collections::{HashMap, HashSet};
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;

use tine_ir::{self as ir, Typed};
use tine_symbols::symbols::*;
use tine_types::{store::TypeStore, types};

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

pub fn ident_from_str(name: &str) -> swc::Ident {
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
pub fn create_num(value: f64) -> swc::Expr {
    swc::Expr::Lit(swc::Lit::Num(swc::Number {
        span: DUMMY_SP,
        value,
        raw: None,
    }))
}
pub fn create_bool(value: bool) -> swc::Expr {
    swc::Expr::Lit(swc::Lit::Bool(swc::Bool {
        span: DUMMY_SP,
        value,
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

pub fn is_primitive(ty: types::TypeId) -> bool {
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
    !is_primitive(node.ty())
}

pub fn undefined() -> swc::Expr {
    swc::Expr::Ident(swc::Ident {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        sym: "undefined".into(),
        optional: false,
    })
}

pub fn internal_method_call(name: &str, args: Vec<swc::ExprOrSpread>) -> swc::CallExpr {
    swc::CallExpr {
        callee: swc::Callee::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(swc::Expr::Ident(ident_from_str("$"))),
            prop: swc::MemberProp::Ident(ident_from_str(name).into()),
        }))),
        args,
        ..Default::default()
    }
}
// TODO: fields
pub fn internal_construct(name: &str) -> swc::Expr {
    swc::Expr::New(swc::NewExpr {
        callee: Box::new(swc::Expr::Member(swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(swc::Expr::Ident(ident_from_str("$"))),
            prop: swc::MemberProp::Ident(ident_from_str(name).into()),
        })),
        ..Default::default()
    })
}

impl CodeGenerator<'_, '_> {
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
                obj: Box::new(swc::Expr::Ident(ident_from_str("$"))),
                prop: swc::MemberProp::Ident(ident_from_str("Option").into()),
            })),
            args: Some(args),
            type_args: None,
        }
    }

    pub fn generate_constructor_name(
        &self,
        constructor_id: SymbolId,
        ty_args: &HashMap<types::TypeParam, types::TypeId>,
    ) -> swc::Expr {
        let name = self.symbols.get_symbol(constructor_id).name();
        let ty = ident_from_str(name);
        match ty_args.len() {
            0 => ty.into(),
            _ => member(
                ty.into(),
                &args_to_string(&ty_args.iter().map(|(_, ty)| *ty).collect::<Vec<_>>()),
            )
            .into(),
        }
    }
}

/// Convert a `Vec<TypeId>` into a unique `String` that will not collide with
/// user-defined names
pub fn args_to_string(args: &[types::TypeId]) -> String {
    let str = args
        .into_iter()
        .map(|a| a.to_string())
        .collect::<Vec<_>>()
        .join("_");
    format!("${}", str)
}

pub fn member(object: swc::Expr, prop: &str) -> swc::MemberExpr {
    swc::MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(object),
        prop: swc::MemberProp::Ident(ident_from_str(prop).into()),
    }
}
pub fn index(object: swc::Expr, i: usize) -> swc::MemberExpr {
    swc::MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(object),
        prop: swc::MemberProp::Computed(swc::ComputedPropName {
            span: DUMMY_SP,
            expr: Box::new(swc::Expr::Lit(swc::Lit::Num(swc::Number {
                span: DUMMY_SP,
                value: i as f64,
                raw: None,
            }))),
        }),
    }
}

pub fn call(callee: swc::Expr, args: Vec<swc::Expr>) -> swc::CallExpr {
    swc::CallExpr {
        callee: swc::Callee::Expr(Box::new(callee)),
        args: args.into_iter().map(Into::into).collect(),
        ..Default::default()
    }
}

pub fn option(value: swc::Expr) -> swc::Expr {
    let callee = swc::Callee::Expr(Box::new(
        member(member(ident_from_str("$").into(), "Option").into(), "$from").into(),
    ));

    swc::Expr::Call(swc::CallExpr {
        callee,
        args: vec![value.into()],
        ..Default::default()
    })
}
