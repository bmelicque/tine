mod unary_type;

use pest::iterators::Pair;

use crate::ast;

use super::{parser::Rule, ParserEngine};

impl ParserEngine {
    pub fn parse_type(&mut self, pair: Pair<'_, Rule>) -> ast::Type {
        match pair.as_rule() {
            Rule::type_annotation
            | Rule::type_element
            | Rule::type_body_element
            | Rule::binary_type
            | Rule::primary_type
            | Rule::grouped_type
            | Rule::type_name => self.parse_type(pair.into_inner().next().unwrap()),
            Rule::function_type => self.parse_function_type(pair).into(),
            Rule::tuple_type => self.parse_tuple_type(pair).into(),
            Rule::map_type => self.parse_map_type(pair).into(),
            Rule::result_type => self.parse_result_type(pair).into(),
            Rule::generic_type => self.parse_named_type_with_args(pair).into(),
            Rule::type_identifier | Rule::primitive_type => self.parse_named_type(pair).into(),
            Rule::unary_type => self.parse_unary_type(pair),
            _ => unreachable!("Unexpected rule '{:?}'", pair.as_rule()),
        }
    }

    pub fn parse_tuple_type(&mut self, pair: Pair<'_, Rule>) -> ast::TupleType {
        assert!(pair.as_rule() == Rule::tuple_type);
        let loc = self.localize(pair.as_span());
        let elements = pair
            .into_inner()
            .map(|pair| self.parse_type(pair))
            .collect();
        return ast::TupleType { loc, elements };
    }

    pub fn parse_map_type(&mut self, pair: Pair<'_, Rule>) -> ast::MapType {
        assert!(pair.as_rule() == Rule::map_type);
        let loc = self.localize(pair.as_span());
        let mut key = None;
        let mut value = None;

        for sub_pair in pair.into_inner() {
            match sub_pair.as_rule() {
                Rule::map_type_key => {
                    let ty = self.parse_type(sub_pair.into_inner().next().unwrap());
                    key = Some(Box::new(ty));
                }
                Rule::map_type_value => {
                    let ty = self.parse_type(sub_pair.into_inner().next().unwrap());
                    value = Some(Box::new(ty));
                }
                _ => unreachable!(
                    "Map type should contain at most a map_type_key and a map_type_value"
                ),
            }
        }

        ast::MapType { loc, key, value }
    }

    fn parse_result_type(&mut self, pair: Pair<'_, Rule>) -> ast::ResultType {
        assert!(pair.as_rule() == Rule::result_type);
        let loc = self.localize(pair.as_span());
        let mut ok = None;
        let mut error = None;

        for sub_pair in pair.into_inner() {
            match sub_pair.as_rule() {
                Rule::result_error_type => {
                    let ty = self.parse_type(sub_pair.into_inner().next().unwrap());
                    error = Some(Box::new(ty));
                }
                Rule::result_ok_type => {
                    let ty = self.parse_type(sub_pair.into_inner().next().unwrap());
                    ok = Some(Box::new(ty));
                }
                _ => unreachable!(
                    "Result typeq should contain at most a result_ok_type and a result_error_type"
                ),
            }
        }

        ast::ResultType { loc, error, ok }
    }

    pub fn parse_named_type_with_args(&mut self, pair: Pair<'_, Rule>) -> ast::NamedType {
        assert!(pair.as_rule() == Rule::generic_type);
        let loc = self.localize(pair.as_span());

        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_str().into();

        let mut args = Vec::new();
        for element_pair in inner {
            args.push(self.parse_type(element_pair));
        }
        let args = if args.len() > 0 { Some(args) } else { None };

        ast::NamedType { loc, name, args }
    }

    pub fn parse_named_type(&mut self, pair: Pair<'_, Rule>) -> ast::NamedType {
        let loc = self.localize(pair.as_span());
        ast::NamedType {
            loc,
            name: pair.as_str().into(),
            args: None,
        }
    }

    fn parse_function_type(&mut self, pair: Pair<'_, Rule>) -> ast::FunctionType {
        assert!(pair.as_rule() == Rule::function_type);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();

        let params = inner
            .next()
            .unwrap()
            .into_inner()
            .map(|sub_pair| self.parse_type(sub_pair))
            .collect();

        let returned = Box::new(self.parse_type(inner.next().unwrap()));

        ast::FunctionType {
            loc,
            params,
            returned,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{Rule, TineParser};
    use pest::Parser;

    fn parse_type_input(input: &'static str, rule: Rule) -> ast::Type {
        let pair = TineParser::parse(rule, input).unwrap().next().unwrap();
        let mut parser_engine = ParserEngine::new(0);
        parser_engine.parse_type(pair)
    }

    #[test]
    fn test_parse_named_type() {
        let input = "int";
        let result = parse_type_input(input, Rule::primitive_type);

        match result {
            ast::Type::Named(named) => {
                assert_eq!(named.name, "int");
                assert!(named.args.is_none());
            }
            _ => panic!("Expected NamedType"),
        }
    }

    #[test]
    fn test_parse_generic_type() {
        let input = "Box<int>";
        let result = parse_type_input(input, Rule::generic_type);

        match result {
            ast::Type::Named(named) => {
                assert_eq!(named.name, "Box");
                assert!(named.args.is_some());
                let args = named.args.unwrap();
                assert_eq!(args.len(), 1);
                match &args[0] {
                    ast::Type::Named(arg_named) => assert_eq!(arg_named.name, "int"),
                    _ => panic!("Expected NamedType as generic argument"),
                }
            }
            _ => panic!("Expected NamedType"),
        }
    }

    #[test]
    fn test_parse_map_type() {
        let input = "str#int";
        let result = parse_type_input(input, Rule::map_type);

        match result {
            ast::Type::Map(map) => {
                match *map.key.unwrap() {
                    ast::Type::Named(named) => assert_eq!(named.name, "str"),
                    _ => panic!("Expected NamedType as map key"),
                }
                match *map.value.unwrap() {
                    ast::Type::Named(named) => assert_eq!(named.name, "int"),
                    _ => panic!("Expected NamedType as map value"),
                }
            }
            _ => panic!("Expected MapType"),
        }
    }

    #[test]
    fn test_parse_tuple_type() {
        let input = "(int, str)";
        let result = parse_type_input(input, Rule::type_annotation);

        match result {
            ast::Type::Tuple(tuple) => {
                assert_eq!(tuple.elements.len(), 2);
                match &tuple.elements[0] {
                    ast::Type::Named(named) => assert_eq!(named.name, "int"),
                    _ => panic!("Expected NamedType as first tuple element"),
                }
                match &tuple.elements[1] {
                    ast::Type::Named(named) => assert_eq!(named.name, "str"),
                    _ => panic!("Expected NamedType as second tuple element"),
                }
            }
            _ => panic!("Expected TupleType"),
        }
    }

    #[test]
    fn test_parse_function_type() {
        let input = "(int, str) -> bool";
        let result = parse_type_input(input, Rule::function_type);

        match result {
            ast::Type::Function(function) => {
                assert_eq!(function.params.len(), 2);
                match &function.params[0] {
                    ast::Type::Named(named) => assert_eq!(named.name, "int"),
                    _ => panic!("Expected NamedType as first function parameter"),
                }
                match &function.params[1] {
                    ast::Type::Named(named) => assert_eq!(named.name, "str"),
                    _ => panic!("Expected NamedType as second function parameter"),
                }
                match *function.returned {
                    ast::Type::Named(named) => assert_eq!(named.name, "bool"),
                    _ => panic!("Expected NamedType as function return type"),
                }
            }
            _ => panic!("Expected FunctionType"),
        }
    }

    #[test]
    fn test_parse_function_type_no_args() {
        let input = "() -> bool";
        let result = parse_type_input(input, Rule::function_type);

        match result {
            ast::Type::Function(function) => match *function.returned {
                ast::Type::Named(named) => assert_eq!(named.name, "bool"),
                _ => panic!("Expected NamedType as function return type"),
            },
            _ => panic!("Expected FunctionType"),
        }
    }

    #[test]
    fn test_parse_result_type() {
        let input = "str!int";
        let result = parse_type_input(input, Rule::result_type);

        match result {
            ast::Type::Result(result_type) => {
                match *result_type.ok.unwrap() {
                    ast::Type::Named(named) => assert_eq!(named.name, "int"),
                    _ => panic!("Expected NamedType as result ok type"),
                }
                match *result_type.error.unwrap() {
                    ast::Type::Named(named) => assert_eq!(named.name, "str"),
                    _ => panic!("Expected NamedType as result error type"),
                }
            }
            _ => panic!("Expected ResultType"),
        }
    }
}
