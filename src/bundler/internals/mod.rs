use swc_bundler::ModuleData;
use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_parser::{lexer::Lexer, EsSyntax, Parser, StringInput, Syntax};

pub fn parse_internals() -> ModuleData {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(
        FileName::Custom("internals".into()).into(),
        include_str!("internals.js"),
    );

    let lexer = Lexer::new(
        Syntax::Es(EsSyntax {
            jsx: false,
            export_default_from: true,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );

    let mut parser = Parser::new_from(lexer);

    let module = parser
        .parse_module()
        .expect("Failed to parse the standard library");

    ModuleData {
        fm,
        module,
        helpers: Default::default(),
    }
}
