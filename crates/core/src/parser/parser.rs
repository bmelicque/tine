use pest::iterators::Pairs;
use pest::Parser;
use pest_derive::Parser;

use crate::ast;
use crate::locations::Span;

#[derive(Parser)]
#[grammar = "parser/grammar.pest"]
pub struct MyLanguageParser;

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
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

    pub fn parse(&mut self, input: &str) -> ParseResult {
        match MyLanguageParser::parse(Rule::program, input) {
            Ok(pairs) => self.build_ast(input, pairs),
            Err(err) => make_invalid_program(input, err),
        }
    }

    fn build_ast<'i>(&mut self, input: &'i str, pairs: Pairs<'i, Rule>) -> ParseResult {
        let span = Span::new(0, input.len() as u32);
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

    pub fn error(&mut self, message: String, span: Span) {
        self.errors.push(ParseError { message, span });
    }
}

fn make_invalid_program(input: &str, err: pest::error::Error<Rule>) -> ParseResult {
    let span = Span::new(0, input.len() as u32);
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
