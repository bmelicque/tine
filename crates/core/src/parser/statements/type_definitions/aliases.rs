use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, ParserEngine},
};

impl ParserEngine {
    pub fn parse_type_alias(&mut self, pair: Pair<'_, Rule>) -> ast::TypeAlias {
        debug_assert_eq!(pair.as_rule(), Rule::type_alias);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();

        let name = inner.next().unwrap().as_str().to_string();
        let mut params = None;
        let mut definition = None;
        for pair in inner {
            match pair.as_rule() {
                Rule::type_params => {
                    params = Some(self.parse_type_params(pair));
                }
                Rule::type_annotation => {
                    definition = Some(self.parse_type(pair));
                }
                _ => unreachable!(),
            }
        }
        let definition = Box::new(definition.unwrap());

        ast::TypeAlias {
            loc,
            name,
            params,
            definition,
        }
    }
}
