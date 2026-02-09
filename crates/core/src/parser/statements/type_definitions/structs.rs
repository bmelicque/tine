use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, ParserEngine},
};

impl ParserEngine {
    pub fn parse_struct_definition(&mut self, pair: Pair<'_, Rule>) -> ast::StructDefinition {
        debug_assert_eq!(pair.as_rule(), Rule::struct_definition);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();

        let name = Some(self.parse_identifier(inner.next().unwrap()));

        let mut params = None;
        let mut body = None;
        for pair in inner {
            match pair.as_rule() {
                Rule::type_params => {
                    params = Some(self.parse_type_params(pair));
                }
                Rule::type_body => {
                    body = Some(self.parse_type_body(pair));
                }
                _ => unreachable!(),
            }
        }

        ast::StructDefinition {
            docs: None,
            loc,
            name,
            params,
            body,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        diagnostics::Diagnostic,
        parser::parser::{Rule, TineParser},
    };
    use pest::Parser;

    fn parse_struct_input(input: &'static str) -> (ast::StructDefinition, Vec<Diagnostic>) {
        let result = TineParser::parse(Rule::struct_definition, input);
        let Ok(mut pair) = result else {
            panic!("Failed to parse input: {:?}", result.err().unwrap());
        };
        let pair = pair.next().unwrap();
        let mut parser_engine = ParserEngine::new(0);
        (
            parser_engine.parse_struct_definition(pair),
            parser_engine.diagnostics,
        )
    }

    #[test]
    fn test_parse_tuple_struct() {
        let input = "struct Box(bool)";
        let (result, errors) = parse_struct_input(input);

        assert!(errors.len() == 0);

        assert_eq!(result.name.unwrap().as_str(), "Box");
        assert!(result.params.is_none());

        match result.body {
            Some(ast::TypeBody::Tuple(tuple)) => {
                assert_eq!(tuple.elements.len(), 1);
            }
            _ => panic!("Expected tuple body"),
        }
    }

    #[test]
    fn test_parse_struct_struct() {
        let input = "struct Box { value bool }";
        let (result, errors) = parse_struct_input(input);

        assert!(errors.len() == 0);

        assert_eq!(result.name.unwrap().as_str(), "Box");
        assert!(result.params.is_none());

        match result.body {
            Some(ast::TypeBody::Struct(st)) => {
                assert_eq!(st.fields.len(), 1);
                assert_eq!(st.fields[0].as_name(), "value");
            }
            _ => panic!("Expected struct body"),
        }
    }

    #[test]
    fn test_parse_generic_def() {
        let input = "struct Box<T>(T)";
        let (result, errors) = parse_struct_input(input);

        assert!(errors.len() == 0);

        assert_eq!(result.name.unwrap().as_str(), "Box");
        assert!(result.params.is_some());
        let params = result.params.unwrap();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], "T");
    }

    #[test]
    fn test_parse_type_alias_with_duplicate_params() {
        let input = "struct Box<T, T>(T)";
        let (result, errors) = parse_struct_input(input);

        assert_eq!(result.name.unwrap().as_str(), "Box");
        assert!(result.params.is_some());
        let params = result.params.unwrap();
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], "T");
        assert_eq!(params[1], "T");

        // Check for errors
        assert!(!errors.is_empty());
        assert_eq!(errors.len(), 1, "expected 1 error, got {}", errors.len());
    }

    #[test]
    fn test_parse_struct_with_duplicate_fields() {
        let input = "struct MyStruct{ field bool, field str }";
        let (_, errors) = parse_struct_input(input);

        assert!(!errors.is_empty());
        assert_eq!(errors.len(), 1);
    }
}
