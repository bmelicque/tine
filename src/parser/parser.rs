use pest::iterators::{Pair, Pairs};
use pest::Parser;
use pest_derive::Parser;

use crate::ast::{AstNode, Node, Spanned};

use super::statements::parse_statement;

#[derive(Parser)]
#[grammar = "parser/grammar.pest"]
pub struct MyLanguageParser;

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: pest::Span<'static>,
}

pub struct ParseResult {
    pub node: Option<AstNode>,
    pub errors: Vec<ParseError>,
}

impl ParseResult {
    pub fn empty() -> Self {
        ParseResult {
            node: None,
            errors: vec![],
        }
    }
}

pub struct ParserEngine;

impl ParserEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, input: &'static str) -> ParseResult {
        match MyLanguageParser::parse(Rule::program, input) {
            Ok(pairs) => self.build_ast(pairs),
            Err(err) => panic!("{}", err),
        }
    }

    fn build_ast(&self, pairs: Pairs<'static, Rule>) -> ParseResult {
        let mut errors = Vec::new();
        let mut nodes = Vec::new();

        for pair in pairs {
            if pair.as_rule() != Rule::program {
                continue;
            }
            for inner in pair.into_inner() {
                let mut result = self.parse_statement(inner);
                if let Some(node) = result.node {
                    nodes.push(node);
                }
                errors.append(&mut result.errors);
            }
        }

        let span = nodes
            .first()
            .map(|n| n.span.clone())
            .unwrap_or_else(|| pest::Span::new("", 0, 0).unwrap());

        ParseResult {
            node: Some(Spanned {
                node: Node::Program(nodes),
                span,
            }),
            errors,
        }
    }

    fn parse_statement(&self, pair: Pair<'static, Rule>) -> ParseResult {
        parse_statement(pair)
    }
}

// fn merge_span<'i>(a: pest::Span<'i>, b: pest::Span<'i>) -> pest::Span<'i> {
//     let start = if a.start() < b.start() {
//         a.start()
//     } else {
//         b.start()
//     };
//     let end = if a.end() < b.end() { a.end() } else { b.end() };
//     pest::Span::new(a.as_str(), start, end).unwrap()
// }
