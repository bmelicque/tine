use crate::{ast, parser2::Parser, DiagnosticKind};

impl Parser<'_> {
    pub fn parse_function_definition(
        &mut self,
        docs: Option<ast::Docs>,
    ) -> ast::FunctionDefinition {
        let definition = self.parse_function_expression();
        if definition.name.is_none() {
            let loc = definition.loc.nth_char(2);
            self.error(DiagnosticKind::MissingName, loc);
        }
        ast::FunctionDefinition { docs, definition }
    }
}
