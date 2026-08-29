mod assignments;
mod enum_definitions;
mod implementations;
mod struct_definitions;
mod trait_definitions;
mod variable_declarations;

use tine_ast as ast;
use tine_ir as ir;
use tine_symbols::symbols::*;
use tine_types::{store::TypeStore, types};

use super::TypeChecker;

impl TypeChecker {
    pub fn visit_statement(&mut self, node: ast::Statement) -> Vec<ir::Statement> {
        match node {
            ast::Statement::Assignment(node) => self
                .visit_assignment(node)
                .map_or(vec![], |a| vec![a.into()]),
            ast::Statement::Break(node) => self
                .visit_break_statement(node)
                .map_or(vec![], |s| vec![s.into()]),
            ast::Statement::Comment(_) => vec![],
            ast::Statement::Continue(node) => {
                vec![ir::Statement::Continue(ir::ContinueStatement {
                    loc: node.loc,
                })]
            }
            ast::Statement::Enum(node) => self.visit_enum_definition(node),
            ast::Statement::Expression(node) => self
                .visit_expression(*node.expression)
                .map_or(vec![], |e| vec![e.into()]),
            ast::Statement::Function(node) => self
                .visit_function_definition(node)
                .into_iter()
                .map(|n| n.into())
                .collect(),
            ast::Statement::Implementation(node) => self.visit_implementation(node),
            ast::Statement::Invalid(_) => vec![],
            ast::Statement::Return(node) => self
                .visit_return_statement(node)
                .map_or(vec![], |s| vec![s.into()]),
            ast::Statement::StructDefinition(node) => self.visit_struct_definition(node),
            ast::Statement::Trait(node) => {
                self.visit_trait_definition(node);
                vec![]
            }
            ast::Statement::TypeAlias(node) => {
                self.visit_type_alias(node);
                vec![]
            }
            ast::Statement::VariableDeclaration(node) => self
                .visit_variable_declaration(node)
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }

    fn visit_break_statement(&mut self, node: ast::BreakStatement) -> Option<ir::BreakStatement> {
        let expression = match node.value {
            Some(value) => Some(self.visit_expression(*value)?),
            None => None,
        };
        Some(ir::BreakStatement {
            loc: node.loc,
            expression: expression.map(Box::new),
        })
    }

    fn visit_function_definition(
        &mut self,
        node: ast::FunctionDefinition,
    ) -> Option<ir::FunctionDefinition> {
        let ast::FunctionDefinition {
            docs, definition, ..
        } = node;
        let docs = docs.map(|d| d.text);
        let definition = self.visit_function_expression(definition, docs)?;
        Some(ir::FunctionDefinition {
            loc: definition.loc,
            name: definition.name.map(|(loc, s)| (loc, s.into()))?,
            params: definition.params,
            body: definition.body,
            ty: definition.ty,
        })
    }

    fn visit_return_statement(
        &mut self,
        node: ast::ReturnStatement,
    ) -> Option<ir::ReturnStatement> {
        match node.value {
            Some(value) => Some(ir::ReturnStatement {
                loc: node.loc,
                expression: Some(Box::new(self.visit_expression(*value)?)),
            }),
            None => Some(ir::ReturnStatement {
                loc: node.loc,
                expression: None,
            }),
        }
    }

    pub fn visit_type_alias(&mut self, node: ast::TypeAlias) {
        let (ty, params) = if let Some(definition) = node.definition {
            self.with_type_params(&node.params, |checker, _| checker.visit_type(definition))
        } else {
            (TypeStore::UNKNOWN, vec![])
        };

        let ty = match params.len() {
            0 => ty,
            _ => self.intern(types::GenericDef { params, def: ty }),
        };

        if let Some(name) = node.name {
            self.symbols.insert::<TypeAliasSymbolId>(TypeAliasSymbol {
                name: name.text.clone(),
                ty,
                defined_at: node.loc,
                ..Default::default()
            });
            self.types.add_alias(ty, name.text);
        }
    }
}
