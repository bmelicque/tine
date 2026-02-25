use swc_bundler::ModuleData;
use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_parser::{lexer::Lexer, EsSyntax, Parser, StringInput, Syntax};

fn parse(name: &str, source: &'static str) -> ModuleData {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom(name.into()).into(), source);

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
        .expect(format!("Failed to parse '{}'", name).as_str());

    ModuleData {
        fm,
        module,
        helpers: Default::default(),
    }
}

pub fn parse_internals() -> ModuleData {
    parse("internals", include_str!("internals.js"))
}

pub fn parse_dom() -> ModuleData {
    parse("dom", include_str!("dom.js"))
}

pub fn parse_signals() -> ModuleData {
    parse("signals", include_str!("signals.js"))
}
