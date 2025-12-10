use pest::iterators::Pair;

use super::ParserEngine;
use crate::{ast, parser::parser::Rule};

impl ParserEngine {
    pub fn parse_unary_type(&mut self, pair: Pair<'_, Rule>) -> ast::Type {
        assert_eq!(pair.as_rule(), Rule::unary_type);
        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::array_type => self.parse_array_type(inner).into(),
            Rule::duck_type => self.parse_duck_type(inner).into(),
            Rule::listener_type => self.parse_listener_type(inner).into(),
            Rule::option_type => self.parse_option_type(inner).into(),
            Rule::reference_type => self.parse_reference_type(inner).into(),
            Rule::signal_type => self.parse_signal_type(inner).into(),
            _ => unreachable!(),
        }
    }

    pub fn parse_array_type(&mut self, pair: Pair<'_, Rule>) -> ast::ArrayType {
        assert!(pair.as_rule() == Rule::array_type);
        let span = pair.as_span().into();
        let element = pair
            .into_inner()
            .next()
            .map(|pair| Box::new(self.parse_type(pair)));

        ast::ArrayType { span, element }
    }

    pub fn parse_duck_type(&mut self, pair: Pair<'_, Rule>) -> ast::DuckType {
        assert!(pair.as_rule() == Rule::duck_type);
        let span = pair.as_span().into();
        let like = pair
            .into_inner()
            .next()
            .map(|pair| Box::new(self.parse_type(pair)))
            .unwrap();
        ast::DuckType { span, like }
    }

    fn parse_listener_type(&mut self, pair: Pair<'_, Rule>) -> ast::ListenerType {
        assert!(pair.as_rule() == Rule::listener_type);
        let span = pair.as_span().into();
        let inner_pair = pair.into_inner().next().unwrap();
        let inner = Box::new(self.parse_type(inner_pair));
        ast::ListenerType { span, inner }
    }

    pub fn parse_option_type(&mut self, pair: Pair<'_, Rule>) -> ast::OptionType {
        assert!(pair.as_rule() == Rule::option_type);
        let span = pair.as_span().into();
        let base = pair
            .into_inner()
            .next()
            .map(|pair| Box::new(self.parse_type(pair)));
        ast::OptionType { span, base }
    }

    fn parse_reference_type(&mut self, pair: Pair<'_, Rule>) -> ast::ReferenceType {
        assert!(pair.as_rule() == Rule::reference_type);
        let span = pair.as_span().into();
        let inner = pair.into_inner().next().unwrap();
        let target = Box::new(self.parse_type(inner));
        ast::ReferenceType { span, target }
    }

    fn parse_signal_type(&mut self, pair: Pair<'_, Rule>) -> ast::SignalType {
        assert!(pair.as_rule() == Rule::signal_type);
        let span = pair.as_span().into();
        let inner_pair = pair.into_inner().next().unwrap();
        let inner = Box::new(self.parse_type(inner_pair));
        ast::SignalType { span, inner }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{MyLanguageParser, Rule};
    use pest::Parser;

    fn parse_type_input(input: &'static str) -> ast::Type {
        let pair = MyLanguageParser::parse(Rule::unary_type, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new();
        parser_engine.parse_type(pair)
    }

    #[test]
    fn test_parse_array_type() {
        let input = "[]number";
        let result = parse_type_input(input);

        match result {
            ast::Type::Array(array) => match *array.element.unwrap() {
                ast::Type::Named(named) => assert_eq!(named.name, "number"),
                _ => panic!("Expected NamedType as array element"),
            },
            _ => panic!("Expected ArrayType"),
        }
    }

    #[test]
    fn test_parse_option_type() {
        let input = "?number";
        let result = parse_type_input(input);

        match result {
            ast::Type::Option(option) => match *option.base.unwrap() {
                ast::Type::Named(named) => assert_eq!(named.name, "number"),
                _ => panic!("Expected NamedType as option base"),
            },
            _ => panic!("Expected OptionType"),
        }
    }

    #[test]
    fn test_parse_signal_type() {
        let input = "$number";
        let result = parse_type_input(input);

        match result {
            ast::Type::Signal(option) => match *option.inner {
                ast::Type::Named(named) => assert_eq!(named.name, "number"),
                _ => panic!("Expected NamedType as option base"),
            },
            _ => panic!("Expected OptionType"),
        }
    }
}
