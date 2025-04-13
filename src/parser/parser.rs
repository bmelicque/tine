use pest::iterators::{Pair, Pairs};
use pest::Parser;
use pest_derive::Parser;

use crate::ast::Node;

use super::statements::parse_statement;

#[derive(Parser)]
#[grammar = "parser/grammar.pest"]
pub struct MyLanguageParser;

#[derive(Debug)]
pub struct ParseError<'i> {
    pub message: String,
    pub span: pest::Span<'i>,
}

pub struct ParseResult<'i> {
    pub node: Option<Node>,
    pub errors: Vec<ParseError<'i>>,
}

impl ParseResult<'_> {
    pub fn empty() -> Self {
        ParseResult {
            node: None,
            errors: vec![],
        }
    }

    pub fn ok(node: Option<Node>) -> Self {
        ParseResult {
            node,
            errors: vec![],
        }
    }
}

pub struct ParserEngine;

impl ParserEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn parse<'i>(&self, input: &'i str) -> ParseResult<'i> {
        match MyLanguageParser::parse(Rule::program, input) {
            Ok(pairs) => self.build_ast(pairs),
            Err(err) => panic!("{}", err),
        }
    }

    fn build_ast<'i>(&self, pairs: Pairs<'i, Rule>) -> ParseResult<'i> {
        let mut errors = Vec::new();
        let mut nodes = Vec::new();

        for pair in pairs {
            if pair.as_rule() != Rule::program {
                continue;
            }
            for inner in pair.into_inner() {
                let mut result = self.parse_statement(inner);
                match result.node {
                    Some(n) => nodes.push(n),
                    None => {}
                }
                errors.append(&mut result.errors);
            }
        }

        ParseResult {
            node: Some(Node::Program(nodes)),
            errors,
        }
    }

    fn parse_statement<'i>(&self, pair: Pair<'i, Rule>) -> ParseResult<'i> {
        parse_statement(pair)
    }
}
