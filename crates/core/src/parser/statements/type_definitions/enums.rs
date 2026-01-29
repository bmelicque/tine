use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, ParserEngine},
};

impl ParserEngine {
    pub fn parse_enum_definition(&mut self, pair: Pair<'_, Rule>) -> ast::EnumDefinition {
        debug_assert_eq!(pair.as_rule(), Rule::enum_definition);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();

        let name = inner.next().unwrap().as_str().to_string();
        let mut params = None;
        let mut variants = None;
        for pair in inner {
            match pair.as_rule() {
                Rule::type_params => {
                    params = Some(self.parse_type_params(pair));
                }
                Rule::enum_body => {
                    variants = Some(self.parse_enum_body(pair));
                }
                _ => unreachable!(),
            }
        }
        let variants = variants.unwrap();
        ast::EnumDefinition {
            loc,
            name,
            params,
            variants,
        }
    }

    fn parse_enum_body(&mut self, pair: Pair<'_, Rule>) -> Vec<ast::VariantDefinition> {
        debug_assert_eq!(pair.as_rule(), Rule::enum_body);
        pair.into_inner()
            .map(|variant_pair| self.parse_enum_variant(variant_pair))
            .collect()
    }

    fn parse_enum_variant(&mut self, pair: Pair<'_, Rule>) -> ast::VariantDefinition {
        debug_assert_eq!(pair.as_rule(), Rule::enum_variant);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_str().to_string();
        let body = inner
            .next()
            .map(|body_pair| self.parse_type_body(body_pair));
        ast::VariantDefinition { loc, name, body }
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

    fn parse_enum_input(input: &'static str) -> (ast::EnumDefinition, Vec<Diagnostic>) {
        let pair = TineParser::parse(Rule::enum_definition, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new(0);
        (
            parser_engine.parse_enum_definition(pair),
            parser_engine.diagnostics,
        )
    }

    #[test]
    fn test_parse_sum_type_alias() {
        let input = "enum Bool { True, False }";
        let (result, errors) = parse_enum_input(input);

        assert_eq!(result.name, "Bool");
        assert_eq!(errors.len(), 0);

        assert_eq!(result.variants.len(), 2);
    }
}
