use pest::iterators::Pairs;
use pest::{Parser, Span};
use pest_derive::Parser;

use crate::ast;

#[derive(Parser)]
#[grammar = "parser/grammar.pest"]
pub struct MyLanguageParser;

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: pest::Span<'static>,
}

pub struct ParseResult {
    pub node: ast::Program,
    pub errors: Vec<ParseError>,
}

pub struct ParserEngine {
    pub errors: Vec<ParseError>,
}

impl ParserEngine {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn parse(&mut self, input: &'static str) -> ParseResult {
        match MyLanguageParser::parse(Rule::program, input) {
            Ok(pairs) => self.build_ast(input, pairs),
            Err(err) => make_invalid_program(input, err),
        }
    }

    fn build_ast(&mut self, input: &'static str, pairs: Pairs<'static, Rule>) -> ParseResult {
        let span = pest::Span::new(input, 0, input.len()).unwrap();
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
            node: ast::Program { span, items },
            errors: self.errors.drain(..).collect(),
        }
    }

    pub fn error(&mut self, message: String, span: Span<'static>) {
        self.errors.push(ParseError { message, span });
    }
}

fn make_invalid_program(input: &'static str, err: pest::error::Error<Rule>) -> ParseResult {
    let span = Span::new(input, 0, input.len()).unwrap();
    let invalid = ast::Item::Invalid(ast::InvalidItem { span });
    let program = ast::Program {
        span,
        items: vec![invalid],
    };
    let error = ParseError {
        message: format!("{}", err),
        span,
    };
    ParseResult {
        node: program,
        errors: vec![error],
    }
}
