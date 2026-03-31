use std::collections::HashMap;

use crate::{
    ast,
    diagnostics::DiagnosticKind,
    ir,
    type_checker::TypeChecker,
    types::{DuckType, Type, TypeId},
    Location, TypeStore,
};

impl TypeChecker<'_> {
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
            ty: self.element_type(),
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
                    ir::Expression::Stringliteral(ir::StringLiteral {
                        loc: attribute.loc,
                        value: s,
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
            ast::ElementChild::Expression(e) => self.visit_dom_expression(e),
            ast::ElementChild::Text(t) => Some(ir::Expression::Stringliteral(ir::StringLiteral {
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

    fn visit_dom_expression(&mut self, expr: ast::Expression) -> Option<ir::Expression> {
        let (expr, deps) = self.with_dependencies(|s| s.visit_expression(expr));
        let expr = expr?;
        self.save_reactive_dependencies(&deps, expr.loc());
        self.ctx.add_dependencies(deps);
        Some(expr)
    }

    pub fn element_type(&mut self) -> TypeId {
        self.intern(Type::Duck(DuckType {
            like: TypeStore::ELEMENT,
        }))
    }
}
