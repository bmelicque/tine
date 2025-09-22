use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_ast::{Module, Stmt};
use swc_ecma_parser::{lexer::Lexer, EsConfig, Parser, StringInput, Syntax, TsConfig};

pub fn parse(js_code: &str) -> Vec<Stmt> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("input.js".into()), js_code.into());

    let lexer = Lexer::new(
        Syntax::Es(EsConfig {
            jsx: false,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );

    let mut parser = Parser::new_from(lexer);

    let module = parser.parse_script().expect("Failed to parse JS");

    module.body
}

pub fn parse_ts(ts_code: &str, name: &str) -> Module {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom(name.into()), ts_code.into());

    let lexer = Lexer::new(
        Syntax::Typescript(TsConfig {
            tsx: false,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );

    let mut parser = Parser::new_from(lexer);

    parser.parse_module().expect("Failed to parse TS")
}
