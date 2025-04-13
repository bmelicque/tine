use pest::iterators::{Pair, Pairs};
use pest::Parser;
use pest_derive::Parser;
use std::error::Error;

use crate::ast::Node;

use super::statements::parse_statement;

#[derive(Parser)]
#[grammar = "parser/grammar.pest"]
pub struct MyLanguageParser;

pub struct ParserEngine;

impl ParserEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, input: &str) -> Result<Node, Box<dyn Error>> {
        let pairs = MyLanguageParser::parse(Rule::program, input)?;
        self.build_ast(pairs)
    }

    fn build_ast(&self, pairs: Pairs<Rule>) -> Result<Node, Box<dyn Error>> {
        let mut nodes = Vec::new();

        for pair in pairs {
            if pair.as_rule() != Rule::program {
                continue;
            }
            pair.into_inner()
                .map(|inner| self.parse_statement(inner))
                .filter_map(|x| x)
                .for_each(|node| nodes.push(node));
        }

        Ok(Node::Program(nodes))
    }

    fn parse_statement(&self, pair: Pair<Rule>) -> Option<Node> {
        parse_statement(pair)
    }
}
