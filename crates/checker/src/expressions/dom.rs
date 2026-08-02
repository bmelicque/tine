use std::collections::HashMap;
use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};
use tine_ir as ir;
use tine_types::store::TypeStore;

use crate::TypeChecker;

impl TypeChecker {
    pub fn visit_element_expression(
        &mut self,
        node: ast::ElementExpression,
    ) -> Option<ir::ElementExpression> {
        let loc = node.loc();
        let (tag_name, attributes, children) = match node {
            ast::ElementExpression::Element(e) => (e.tag_name, e.attributes, e.children),
            ast::ElementExpression::Void(e) => (e.tag_name, e.attributes, vec![]),
        };
        let attributes = self.visit_attributes(attributes);
        let children = self.visit_children(children);
        Some(ir::ElementExpression {
            loc,
            tag_name,
            attributes: attributes?,
            children: children?,
            ty: TypeStore::ELEMENT,
        })
    }

    fn visit_attributes(&mut self, attributes: Vec<ast::Attribute>) -> Option<Vec<ir::Attribute>> {
        self.report_duplicated_attributes(&attributes);
        let attributes = attributes
            .into_iter()
            .map(|a| self.visit_attribute(a))
            .collect::<Vec<_>>();
        attributes.into_iter().collect()
    }

    fn report_duplicated_attributes(&mut self, attributes: &Vec<ast::Attribute>) {
        let mut map = HashMap::<String, Vec<Location>>::new();
        for attribute in attributes {
            match map.get_mut(&attribute.name) {
                Some(locs) => locs.push(attribute.loc),
                None => {
                    map.insert(attribute.name.clone(), vec![attribute.loc]);
                }
            };
        }
        for (name, locs) in map {
            if locs.len() == 1 {
                continue;
            }
            let error = DiagnosticKind::DuplicateAttribute { name };
            for loc in locs {
                self.error(error.clone(), loc);
            }
        }
    }

    fn visit_attribute(&mut self, attribute: ast::Attribute) -> Option<ir::Attribute> {
        let value = match attribute.value {
            Some(v) => match v {
                ast::AttributeValue::Expression(e) => self.visit_expression(e)?,
                ast::AttributeValue::String(s) => {
                    ir::Expression::StringLiteral(ir::StringLiteral {
                        loc: attribute.loc,
                        value: s.text,
                    })
                }
            },
            None => ir::Expression::BooleanLiteral(ir::BooleanLiteral {
                loc: attribute.loc,
                value: true,
            }),
        };
        Some(ir::Attribute {
            loc: attribute.loc,
            name: attribute.name,
            value,
        })
    }

    fn visit_children(&mut self, children: Vec<ast::ElementChild>) -> Option<Vec<ir::Expression>> {
        children
            .into_iter()
            .map(|c| self.visit_child(c))
            .collect::<Vec<_>>()
            .into_iter()
            .collect()
    }

    fn visit_child(&mut self, child: ast::ElementChild) -> Option<ir::Expression> {
        match child {
            ast::ElementChild::Expression(e) => self.visit_expression(e),
            ast::ElementChild::Text(t) => Some(ir::Expression::StringLiteral(ir::StringLiteral {
                loc: t.loc,
                value: t.text,
            })),
            ast::ElementChild::Element(e) => {
                self.visit_element_expression(e.into()).map(Into::into)
            }
            ast::ElementChild::VoidElement(v) => {
                self.visit_element_expression(v.into()).map(Into::into)
            }
        }
    }
}
