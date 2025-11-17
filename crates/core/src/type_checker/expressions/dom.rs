use std::{collections::HashMap, sync::OnceLock};

use pest::Span;

use crate::{
    ast,
    type_checker::TypeChecker,
    types::{DuckType, ListenerType, StructType, Type, TypeId},
};

static ELEMENT: OnceLock<TypeId> = OnceLock::new();

impl TypeChecker {
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
        self.analysis_context.save_expression_type(node.span, t)
    }

    fn visit_void_element(&mut self, node: &ast::VoidElement) -> TypeId {
        self.visit_attributes(&node.attributes);
        let t = self.element_type();
        self.analysis_context.save_expression_type(node.span, t)
    }

    fn visit_attributes(&mut self, attributes: &Vec<ast::Attribute>) {
        self.report_duplicated_attributes(attributes);
        self.visit_attributes_values(attributes);
    }

    fn report_duplicated_attributes(&mut self, attributes: &Vec<ast::Attribute>) {
        let mut map = HashMap::<String, Vec<Span<'static>>>::new();
        for attribute in attributes {
            match map.get_mut(&attribute.name) {
                Some(spans) => spans.push(attribute.span),
                None => {
                    map.insert(attribute.name.clone(), vec![attribute.span]);
                }
            };
        }
        for (name, spans) in map {
            if spans.len() == 1 {
                continue;
            }
            let message = format!("Duplicated attribute {}", name);
            for span in spans {
                self.error(message.clone(), span);
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
        let count = self.save_reactive_dependencies(&deps, expr.as_span());
        let is_reactive = self.resolve(expr_type).is_reactive();
        if count > 0 && !is_reactive {
            expr_type = self
                .analysis_context
                .type_store
                .add(Type::Listener(ListenerType { inner: expr_type }));
        }
        self.analysis_context.add_dependencies(deps);
        return expr_type;
    }

    pub fn element_type(&mut self) -> TypeId {
        match ELEMENT.get() {
            Some(t) => *t,
            None => {
                let st = self
                    .analysis_context
                    .type_store
                    .add(Type::Struct(StructType {
                        id: self.analysis_context.type_store.get_next_id(),
                        fields: vec![],
                    }));
                let t = self
                    .analysis_context
                    .type_store
                    .add(Type::Duck(DuckType { like: st }));
                let _ = ELEMENT.set(t);
                t
            }
        }
    }
}
