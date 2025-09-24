use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_ast::Module;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};

pub fn parse_std() -> Module {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(
        FileName::Custom("internals".into()).into(),
        include_str!("internals.ts"),
    );

    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: false,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );

    let mut parser = Parser::new_from(lexer);

    parser
        .parse_module()
        .expect("Failed to parse the standard library")
}
