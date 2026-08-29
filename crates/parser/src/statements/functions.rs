use tine_ast as ast;
use tine_common::{diagnostics::DiagnosticKind, locations::Location};

use crate::Parser;

impl Parser<'_> {
    pub fn parse_function_definition(
        &mut self,
        docs: Option<ast::Docs>,
        pub_loc: Option<Location>,
    ) -> ast::FunctionDefinition {
        let mut definition = self.parse_function_expression();
        if let Some(loc) = pub_loc {
            definition.loc = Location::merge(loc, definition.loc);
        }
        if definition.name.is_none() {
            let loc = definition.loc.nth_char(2);
            self.error(DiagnosticKind::MissingName, loc);
        }
        let loc = pub_loc.map_or(definition.loc, |l| Location::merge(l, definition.loc));

        definition
            .params
            .iter()
            .flat_map(|p| &p.params)
            .filter(|p| p.type_annotation.is_none())
            .for_each(|p| {
                self.error(DiagnosticKind::ExpectedTypeAnnotation, p.loc.increment());
            });

        ast::FunctionDefinition {
            docs,
            loc,
            public: pub_loc.is_some(),
            definition,
        }
    }
}
