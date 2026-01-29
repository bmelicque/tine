use pest::iterators::Pair;

use crate::{
    ast,
    diagnostics::DiagnosticKind,
    parser::{parser::Rule, ParserEngine},
};

impl ParserEngine {
    pub fn parse_element_expression(&mut self, pair: Pair<'_, Rule>) -> ast::ElementExpression {
        assert_eq!(pair.as_rule(), Rule::element_expression);
        match pair.into_inner().next().unwrap() {
            p if p.as_rule() == Rule::dom_element => self.parse_element(p).into(),
            p if p.as_rule() == Rule::void_element => self.parse_void_element(p).into(),
            _ => unreachable!(),
        }
    }

    fn parse_element(&mut self, pair: Pair<'_, Rule>) -> ast::Element {
        assert_eq!(pair.as_rule(), Rule::dom_element);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let start_tag = inner.next().unwrap();
        let (tag_name, attributes) = self.parse_start_tag(start_tag);
        let mut children = Vec::new();

        for next in inner {
            match next.as_rule() {
                Rule::child => children.push(self.parse_child(next)),
                Rule::end_tag => {
                    self.check_end_tag(next, &tag_name);
                    break;
                }
                _ => unreachable!(),
            }
        }
        ast::Element {
            loc,
            tag_name,
            attributes,
            children,
        }
    }

    fn parse_void_element(&mut self, pair: Pair<'_, Rule>) -> ast::VoidElement {
        assert_eq!(pair.as_rule(), Rule::void_element);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let tag_name = inner.next().unwrap().as_str().to_string();
        let attributes = inner.map(|attr| self.parse_attribute(attr)).collect();
        ast::VoidElement {
            loc,
            tag_name,
            attributes,
        }
    }

    fn parse_start_tag(&mut self, pair: Pair<'_, Rule>) -> (String, Vec<ast::Attribute>) {
        assert_eq!(pair.as_rule(), Rule::start_tag);
        let mut inner = pair.into_inner();
        let tag_name = inner.next().unwrap().as_str().to_string();
        let attributes = inner.map(|attr| self.parse_attribute(attr)).collect();
        (tag_name, attributes)
    }

    fn parse_attribute(&mut self, pair: Pair<'_, Rule>) -> ast::Attribute {
        assert_eq!(pair.as_rule(), Rule::attribute);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_str().to_string();
        let value = inner.next().map(|v| self.parse_attribute_value(v));
        ast::Attribute { loc, name, value }
    }

    fn parse_attribute_value(&mut self, pair: Pair<'_, Rule>) -> ast::AttributeValue {
        assert_eq!(pair.as_rule(), Rule::attr_value);
        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::string_literal => inner.as_str().to_string().into(),
            Rule::expression => self.parse_expression(inner).into(),
            _ => unreachable!(),
        }
    }

    fn parse_child(&mut self, pair: Pair<'_, Rule>) -> ast::ElementChild {
        assert_eq!(pair.as_rule(), Rule::child);
        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::dom_element => self.parse_element(inner).into(),
            Rule::void_element => self.parse_void_element(inner).into(),
            Rule::text => {
                let loc = self.localize(inner.as_span());
                let text = inner.as_str().to_string();
                ast::TextNode { loc, text }.into()
            }
            Rule::code => self
                .parse_expression(inner.into_inner().next().unwrap())
                .into(),
            _ => unreachable!(),
        }
    }

    /** Parse the end tag to make sure it matches the opening tag */
    fn check_end_tag(&mut self, pair: Pair<'_, Rule>, expected: &str) {
        assert_eq!(pair.as_rule(), Rule::end_tag);
        let loc = self.localize(pair.as_span());
        let got = pair.into_inner().next().unwrap().as_str().to_string();
        if got != expected {
            let error = DiagnosticKind::MismatchedTags {
                open: expected.to_string(),
                close: got,
            };
            self.error(error, loc);
        }
    }
}
