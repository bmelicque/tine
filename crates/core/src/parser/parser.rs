use pest::iterators::Pairs;
use pest::Parser;
use pest_derive::Parser;

use crate::analyzer::ModuleId;
use crate::diagnostics::{Diagnostic, DiagnosticKind, DiagnosticLevel};
use crate::locations::Span;
use crate::{ast, Location};

#[derive(Parser)]
#[grammar = "parser/grammar.pest"]
pub struct TineParser;

pub struct ParseResult {
    pub node: ast::Program,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct ParserEngine {
    pub module: ModuleId,
    pub diagnostics: Vec<Diagnostic>,
}

impl ParserEngine {
    pub fn new(module: ModuleId) -> Self {
        Self {
            module,
            diagnostics: Vec::new(),
        }
    }

    pub fn parse(&mut self, input: &str) -> ParseResult {
        match TineParser::parse(Rule::program, input) {
            Ok(pairs) => self.build_ast(input, pairs),
            Err(err) => self.make_invalid_program(input, err),
        }
    }

    fn build_ast<'i>(&mut self, input: &'i str, pairs: Pairs<'i, Rule>) -> ParseResult {
        let span = Span::new(0, input.len() as u32);
        let loc = Location::new(self.module, span);
        let items: Vec<ast::Item> = pairs
            .into_iter()
            .filter(|pair| pair.as_rule() == Rule::program)
            .flat_map(|pair| {
                let statements: Vec<ast::Item> = pair
                    .into_inner()
                    .filter(|p| p.as_rule() == Rule::item)
                    .map(|pair| self.parse_item(pair))
                    .collect();
                statements
            })
            .collect();

        ParseResult {
            node: ast::Program { loc, items },
            diagnostics: self.diagnostics.drain(..).collect(),
        }
    }

    fn make_invalid_program(&self, input: &str, err: pest::error::Error<Rule>) -> ParseResult {
        let span = Span::new(0, input.len() as u32);
        let loc = Location::new(self.module, span);

        let invalid = ast::Item::Invalid(ast::InvalidItem { loc });
        let program = ast::Program {
            loc,
            items: vec![invalid],
        };
        let error = Diagnostic {
            level: DiagnosticLevel::Error,
            loc,
            kind: DiagnosticKind::ParseError(format!("{}", err)),
        };
        ParseResult {
            node: program,
            diagnostics: vec![error],
        }
    }

    pub fn localize(&self, span: pest::Span<'_>) -> Location {
        Location::new(self.module, span.into())
    }

    pub fn error(&mut self, kind: DiagnosticKind, loc: Location) {
        self.diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Error,
            loc,
            kind,
        });
    }
}
