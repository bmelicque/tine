use tine_ast::*;
use tine_common::diagnostics::DiagnosticKind;

use crate::expander::Expander;

impl Expander {
    pub fn expand_statement(&mut self, node: Statement) -> Vec<Statement> {
        use Statement::*;
        match node {
            Assignment(mut a) => {
                a.value = a.value.map(|v| self.expand_expression(v));
                vec![Assignment(a)]
            }
            Break(mut b) => {
                b.value = b.value.map(|v| Box::new(self.expand_expression(*v)));
                vec![Break(b)]
            }
            Enum(e) => self.expand_enum_def(e),
            Expression(mut e) => {
                e.expression = self.expand_expression(*e.expression).into();
                vec![Expression(e)]
            }
            Function(mut f) => {
                f.definition = self.expand_function(f.definition);
                vec![Function(f)]
            }
            Implementation(i) => vec![self.expand_implementation(i).into()],
            Return(mut r) => {
                r.value = r.value.map(|v| Box::new(self.expand_expression(*v)));
                vec![Return(r)]
            }
            StructDefinition(s) => self.expand_struct_def(s),
            VariableDeclaration(mut v) => {
                v.value = v.value.map(|v| self.expand_expression(v));
                vec![VariableDeclaration(v)]
            }
            no_expand => vec![no_expand],
        }
    }

    fn expand_implementation(&mut self, mut node: Implementation) -> Implementation {
        node.body = node.body.map(|mut b| {
            b.items = b
                .items
                .into_iter()
                .map(|i| self.expand_implementation_item(i))
                .collect();
            b
        });
        node
    }

    fn expand_implementation_item(&mut self, node: ImplementationItem) -> ImplementationItem {
        match node {
            ImplementationItem::Method(mut m) => {
                m.body = m.body.map(|b| self.expand_block(b));
                m.into()
            }
            ImplementationItem::StaticMethod(mut m) => {
                m.definition = self.expand_function(m.definition);
                m.into()
            }
        }
    }

    fn expand_struct_def(&mut self, node: StructDefinition) -> Vec<Statement> {
        let Some(meta) = &node.meta else {
            return vec![node.into()];
        };
        let Some(name) = &node.name else {
            return vec![node.into()];
        };

        let items = meta
            .into_iter()
            .filter_map(|m| Some((m.name.clone()?, m.args.clone())))
            .flat_map(|(n, args)| match n.as_str() {
                "derive" => self.derive_struct(n, args, &node.body),
                _ => {
                    self.error(n.loc, DiagnosticKind::UnknownMacro);
                    vec![]
                }
            })
            .collect();
        let body = ImplementationBody {
            loc: node.loc,
            items,
        };

        let implementation = Statement::Implementation(Implementation {
            loc: node.loc,
            implemented_type: Some(Identifier::new(name.text.clone(), node.loc).into()),
            body: Some(body),
        });

        vec![node.into(), implementation]
    }

    fn expand_enum_def(&mut self, node: EnumDefinition) -> Vec<Statement> {
        let Some(meta) = &node.meta else {
            return vec![node.into()];
        };
        let Some(name) = &node.name else {
            return vec![node.into()];
        };

        let items = meta
            .into_iter()
            .filter_map(|m| Some((m.name.clone()?, m.args.clone())))
            .flat_map(|(n, args)| match n.as_str() {
                "derive" => self.derive_enum(n, args, &node),
                _ => {
                    self.error(n.loc, DiagnosticKind::UnknownMacro);
                    vec![]
                }
            })
            .collect();
        let body = ImplementationBody {
            loc: node.loc,
            items,
        };

        let implementation = Statement::Implementation(Implementation {
            loc: node.loc,
            implemented_type: Some(Identifier::new(name.text.clone(), node.loc).into()),
            body: Some(body),
        });

        vec![node.into(), implementation]
    }
}
