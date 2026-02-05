use crate::{ast, parser2::Parser};

impl Parser<'_> {
    pub fn parse_function_definition(
        &mut self,
        docs: Option<ast::Docs>,
    ) -> ast::FunctionDefinition {
        unimplemented!()
    }
}
