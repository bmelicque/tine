use pest::iterators::{Pair, Pairs};
use pest::Parser;
use pest_derive::Parser;
use std::error::Error;

use crate::ast::Node;

#[derive(Parser)]
#[grammar = "grammar.pest"]
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
            match pair.as_rule() {
                Rule::program => {
                    for inner_pair in pair.into_inner() {
                        if let Some(node) = self.parse_statement(inner_pair) {
                            nodes.push(node);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(Node::Program(nodes))
    }

    fn parse_statement(&self, pair: Pair<Rule>) -> Option<Node> {
        match pair.as_rule() {
            Rule::statement => {
                let inner_pair = pair.into_inner().next().unwrap();
                self.parse_statement(inner_pair)
            }
            Rule::variable_declaration => self.parse_variable_declaration(pair),
            Rule::function_declaration => self.parse_function_declaration(pair),
            Rule::return_statement => self.parse_return_statement(pair),
            _ => None,
        }
    }

    // Parse a variable declaration
    fn parse_variable_declaration(&self, pair: Pair<Rule>) -> Option<Node> {
        let mut inner = pair.into_inner();
        let name = inner.next()?.as_str().to_string();

        let mut type_annotation = None;
        let mut initializer = None;

        for item in inner {
            match item.as_rule() {
                Rule::type_annotation => {
                    if let Some(type_name_pair) = item.into_inner().next() {
                        type_annotation = Some(type_name_pair.as_str().to_string());
                    }
                }
                Rule::expression => {
                    initializer = self.parse_expression(item).map(Box::new);
                }
                _ => {}
            }
        }

        Some(Node::VariableDeclaration {
            name,
            type_annotation,
            initializer,
        })
    }

    // Parse a function declaration
    fn parse_function_declaration(&self, pair: Pair<Rule>) -> Option<Node> {
        let mut inner = pair.into_inner();
        let name = inner.next()?.as_str().to_string();

        let mut params = Vec::new();
        let mut return_type = None;
        let mut body = Vec::new();

        for item in inner {
            match item.as_rule() {
                Rule::parameter_list => {
                    for param in item.into_inner() {
                        let mut param_inner = param.into_inner();
                        let param_name = param_inner.next()?.as_str().to_string();
                        let type_annotation = param_inner.next()?;
                        let param_type = type_annotation.into_inner().next()?.as_str().to_string();
                        params.push((param_name, param_type));
                    }
                }
                Rule::type_annotation => {
                    if let Some(type_name_pair) = item.into_inner().next() {
                        return_type = Some(type_name_pair.as_str().to_string());
                    }
                }
                Rule::block => {
                    for stmt in item.into_inner() {
                        if let Some(node) = self.parse_statement(stmt) {
                            body.push(node);
                        }
                    }
                }
                _ => {}
            }
        }

        Some(Node::FunctionDeclaration {
            name,
            params,
            return_type,
            body,
        })
    }

    // Parse a return statement
    fn parse_return_statement(&self, pair: Pair<Rule>) -> Option<Node> {
        let inner = pair.into_inner().next();
        let expr = inner.and_then(|p| self.parse_expression(p).map(Box::new));

        Some(Node::ReturnStatement(expr))
    }

    // Parse an expression
    fn parse_expression(&self, pair: Pair<Rule>) -> Option<Node> {
        match pair.as_rule() {
            Rule::expression => self.parse_expression(pair.into_inner().next()?),
            Rule::primary => self.parse_expression(pair.into_inner().next()?),
            Rule::binary_expression => {
                let mut inner = pair.into_inner();
                let left = self.parse_expression(inner.next()?)?;
                let operator = inner.next()?.as_str().to_string();
                let right = self.parse_expression(inner.next()?)?;

                Some(Node::BinaryExpression {
                    left: Box::new(left),
                    operator,
                    right: Box::new(right),
                })
            }
            Rule::identifier => Some(Node::Identifier(pair.as_str().to_string())),
            Rule::string_literal => {
                let value = pair.as_str();
                Some(Node::StringLiteral(value[1..value.len() - 1].to_string()))
            }
            Rule::number_literal => Some(Node::NumberLiteral(pair.as_str().parse().unwrap_or(0.0))),
            Rule::boolean_literal => Some(Node::BooleanLiteral(pair.as_str() == "true")),
            _ => None,
        }
    }
}
