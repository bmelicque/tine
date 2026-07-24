pub mod eq;

use tine_ast as ast;
use tine_common::diagnostics::DiagnosticKind;

use crate::expander::Expander;

impl Expander {
    pub fn derive_struct(
        &mut self,
        callee: ast::Identifier,
        args: Option<Vec<ast::Identifier>>,
        node: &Option<ast::TypeBody>,
    ) -> Vec<ast::ImplementationItem> {
        let Some(args) = args else {
            self.error(callee.loc, DiagnosticKind::MissingArguments);
            return vec![];
        };

        args.into_iter()
            .filter_map(|arg| match arg.as_str() {
                "Eq" => Some(eq::derive_struct(node, arg.loc)),
                _ => {
                    self.error(arg.loc, DiagnosticKind::UnknownDeriveArgument);
                    None
                }
            })
            .collect::<Vec<_>>()
    }
}
