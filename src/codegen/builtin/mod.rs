mod parse;

pub fn create_element() -> Vec<swc_ecma_ast::Stmt> {
    parse::parse(include_str!("create-element.js"))
}

pub fn reference() -> Vec<swc_ecma_ast::Stmt> {
    parse::parse(include_str!("reference.js"))
}

pub fn reactive() -> swc_ecma_ast::Module {
    parse::parse_ts(include_str!("signals.ts"), "signals.ts")
}
