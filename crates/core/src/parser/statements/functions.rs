use pest::iterators::Pair;

use crate::{
    ast,
    diagnostics::DiagnosticKind,
    parser::{parser::Rule, ParserEngine},
};

impl ParserEngine {
    pub fn parse_function_definition(&mut self, pair: Pair<'_, Rule>) -> ast::FunctionDefinition {
        debug_assert_eq!(pair.as_rule(), Rule::function_definition);
        let mut inner = pair.into_inner();
        let docs = if inner.peek().unwrap().as_rule() == Rule::doc_comment {
            Some(self.parse_docs(inner.next().unwrap()))
        } else {
            None
        };
        let definition = self.parse_function_expression(inner.next().unwrap());
        if definition.name.is_none() {
            self.error(
                DiagnosticKind::MissingFunctionName,
                definition
                    .loc
                    .decrement()
                    .increment()
                    .increment()
                    .increment(),
            );
        }
        ast::FunctionDefinition { docs, definition }
    }
}
