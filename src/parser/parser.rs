use pest::iterators::Pairs;
use pest::Parser;
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
            Ok(pairs) => self.build_ast(pairs),
            Err(err) => panic!("{}", err),
        }
    }

    fn build_ast(&mut self, pairs: Pairs<'static, Rule>) -> ParseResult {
        let statements: Vec<ast::Statement> = pairs
            .into_iter()
            .filter(|pair| pair.as_rule() == Rule::program)
            .flat_map(|pair| {
                let statements: Vec<ast::Statement> = pair
                    .into_inner()
                    .map(|pair| self.parse_statement(pair))
                    .filter(|statement| !statement.is_empty())
                    .collect();
                statements
            })
            .collect();

        ParseResult {
            node: ast::Program { statements },
            errors: self.errors.drain(..).collect(),
        }
    }
}
