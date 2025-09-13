use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, ParserEngine},
};

impl ParserEngine {
    pub fn parse_identifier(&mut self, pair: Pair<'static, Rule>) -> ast::Identifier {
        assert_eq!(pair.as_rule(), Rule::value_identifier);
        ast::Identifier {
            span: pair.as_span(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{MyLanguageParser, Rule};
    use pest::Parser;

    fn parse_expression_input(input: &'static str) -> ast::Expression {
        let pair = MyLanguageParser::parse(Rule::value_identifier, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new();
        parser_engine.parse_expression(pair)
    }

    #[test]
    fn test_parse_identifier() {
        let input = "myVariable";
        let result = parse_expression_input(input);

        match result {
            ast::Expression::Identifier(identifier) => {
                assert_eq!(identifier.span.as_str(), "myVariable");
            }
            _ => panic!("Expected Identifier"),
        }
    }

    #[test]
    fn test_parse_private_identifier() {
        let input = "_private";
        let result = parse_expression_input(input);

        match result {
            ast::Expression::Identifier(identifier) => {
                assert_eq!(identifier.span.as_str(), "_private");
            }
            _ => panic!("Expected Identifier"),
        }
    }

    #[test]
    fn test_parse_identifier_with_numbers() {
        let input = "value2";
        let result = parse_expression_input(input);

        match result {
            ast::Expression::Identifier(identifier) => {
                assert_eq!(identifier.span.as_str(), "value2");
            }
            _ => panic!("Expected Identifier"),
        }
    }
}
