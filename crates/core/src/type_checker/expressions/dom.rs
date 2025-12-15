use std::collections::HashMap;

use crate::{
    ast,
    type_checker::TypeChecker,
    types::{DuckType, ListenerType, Type, TypeId},
    Location, TypeStore,
};

impl TypeChecker<'_> {
    pub fn visit_element_expression(&mut self, node: &ast::ElementExpression) -> TypeId {
        match node {
            ast::ElementExpression::Element(e) => self.visit_element_with_children(e),
            ast::ElementExpression::Void(v) => self.visit_void_element(v),
        }
    }

    fn visit_element_with_children(&mut self, node: &ast::Element) -> TypeId {
        self.visit_attributes(&node.attributes);
        self.visit_children(&node.children);
        let t = self.element_type();
        self.ctx.save_expression_type(node.loc, t)
    }

    fn visit_void_element(&mut self, node: &ast::VoidElement) -> TypeId {
        self.visit_attributes(&node.attributes);
        let t = self.element_type();
        self.ctx.save_expression_type(node.loc, t)
    }

    fn visit_attributes(&mut self, attributes: &Vec<ast::Attribute>) {
        self.report_duplicated_attributes(attributes);
        self.visit_attributes_values(attributes);
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
            let message = format!("Duplicated attribute {}", name);
            for loc in locs {
                self.error(message.clone(), loc);
            }
        }
    }

    fn visit_attributes_values(&mut self, attributes: &Vec<ast::Attribute>) {
        for attribute in attributes {
            if let Some(ast::AttributeValue::Expression(ref expr)) = attribute.value {
                self.visit_dom_expression(expr);
            }
        }
    }

    fn visit_children(&mut self, children: &Vec<ast::ElementChild>) {
        for child in children {
            match child {
                ast::ElementChild::Expression(e) => {
                    self.visit_dom_expression(e);
                }
                ast::ElementChild::Text(_) => {}
                ast::ElementChild::Element(e) => {
                    self.visit_element_with_children(e);
                }
                ast::ElementChild::VoidElement(v) => {
                    self.visit_void_element(v);
                }
            };
        }
    }

    fn visit_dom_expression(&mut self, expr: &ast::Expression) -> TypeId {
        let (mut expr_type, deps) = self.with_dependencies(|s| s.visit_expression(expr));
        let count = self.save_reactive_dependencies(&deps, expr.loc());
        let is_reactive = self.resolve(expr_type).is_reactive();
        if count > 0 && !is_reactive {
            expr_type = self
                .ctx
                .type_store
                .add(Type::Listener(ListenerType { inner: expr_type }));
        }
        self.ctx.add_dependencies(deps);
        return expr_type;
    }

    pub fn element_type(&mut self) -> TypeId {
        self.intern(Type::Duck(DuckType {
            like: TypeStore::ELEMENT,
        }))
    }
}
