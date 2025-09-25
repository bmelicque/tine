use std::collections::HashMap;

use pest::Span;

use crate::{ast, type_checker::TypeChecker, types};

impl TypeChecker {
    pub fn visit_element_expression(&mut self, node: &ast::ElementExpression) -> types::Type {
        match node {
            ast::ElementExpression::Element(e) => self.visit_element_with_children(e),
            ast::ElementExpression::Void(v) => self.visit_void_element(v),
        }
    }

    fn visit_element_with_children(&mut self, node: &ast::Element) -> types::Type {
        self.visit_attributes(&node.attributes);
        self.visit_children(&node.children);
        self.set_type_at(node.span, element_type())
    }

    fn visit_void_element(&mut self, node: &ast::VoidElement) -> types::Type {
        self.visit_attributes(&node.attributes);
        self.set_type_at(node.span, element_type())
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
                ast::ElementChild::Expression(e) => self.visit_dom_expression(e),
                ast::ElementChild::Text(_) => types::Type::String,
                ast::ElementChild::Element(e) => self.visit_element_with_children(e),
                ast::ElementChild::VoidElement(v) => self.visit_void_element(v),
            };
        }
    }

    fn visit_dom_expression(&mut self, expr: &ast::Expression) -> types::Type {
        let (expr_type, deps) = self.with_dependencies(|s| s.visit_expression(expr));
        if expr_type.is_reactive() {
            self.save_reactive_dependencies(&deps, expr.as_span());
        }
        self.analysis_context.add_dependencies(deps);
        return expr_type;
    }
}

fn element_type() -> types::Type {
    types::Type::Duck(types::DuckType {
        like: Box::new(types::Type::Named(types::NamedType {
            name: "Element".into(),
            args: Vec::new(),
        })),
    })
}
