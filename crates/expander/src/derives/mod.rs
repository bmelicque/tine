pub mod eq;
pub mod hash;

use tine_ast::*;
use tine_common::diagnostics::DiagnosticKind;

use crate::expander::Expander;

impl Expander {
    pub fn derive_struct(
        &mut self,
        callee: Identifier,
        args: Option<Vec<Identifier>>,
        node: &StructDefinition,
    ) -> Vec<MethodDefinition> {
        let name = match &node.name {
            Some(name) => name.as_str(),
            None => return vec![],
        };
        let body = &node.body;
        let Some(args) = args else {
            self.error(callee.loc, DiagnosticKind::MissingArguments);
            return vec![];
        };

        args.into_iter()
            .filter_map(|arg| match arg.as_str() {
                "Eq" => Some(eq::derive_struct(name, body, arg.loc)),
                "Hash" => Some(hash::derive_struct(body, arg.loc)),
                _ => {
                    self.error(arg.loc, DiagnosticKind::UnknownDeriveArgument);
                    None
                }
            })
            .collect::<Vec<_>>()
    }

    pub fn derive_enum(
        &mut self,
        callee: Identifier,
        args: Option<Vec<Identifier>>,
        node: &EnumDefinition,
    ) -> Vec<MethodDefinition> {
        let Some(args) = args else {
            self.error(callee.loc, DiagnosticKind::MissingArguments);
            return vec![];
        };

        let name = match &node.name {
            Some(name) => name.as_str(),
            None => return vec![],
        };

        args.into_iter()
            .filter_map(|arg| match arg.as_str() {
                "Eq" => Some(eq::derive_enum(name, &node.items, arg.loc)),
                _ => {
                    self.error(arg.loc, DiagnosticKind::UnknownDeriveArgument);
                    None
                }
            })
            .collect::<Vec<_>>()
    }
}
