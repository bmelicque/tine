use tine_common::locations::{Locatable, Location};
use tine_macros::tree_struct;

use crate::{nodes::ast_enum, walk::PushNodes, StringLiteral};

use super::Expression;

ast_enum!(ElementExpression {
    Element(Element),
    Void(VoidElement),
});

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Element {
    pub tag_name: String,
    #[child]
    pub attributes: Vec<Attribute>,
    #[child]
    pub children: Vec<ElementChild>,
}
impl Into<Expression> for Element {
    fn into(self) -> Expression {
        Expression::Element(self.into())
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct VoidElement {
    pub tag_name: String,
    pub attributes: Vec<Attribute>,
}
impl Into<Expression> for VoidElement {
    fn into(self) -> Expression {
        Expression::Element(self.into())
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TextNode {
    pub text: String,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub name: String,
    #[child]
    pub value: Option<AttributeValue>,
}
impl<'a> PushNodes<'a> for Attribute {
    fn push_nodes(&'a self, stack: &mut Vec<crate::Node<'a>>) {
        self.push_children(stack);
    }
}

ast_enum!(AttributeValue {
    Expression(Expression),
    String(StringLiteral),
});
impl<'a> PushNodes<'a> for AttributeValue {
    fn push_nodes(&'a self, stack: &mut Vec<crate::Node<'a>>) {
        match self {
            AttributeValue::String(_) => {}
            AttributeValue::Expression(e) => e.push_nodes(stack),
        }
    }
}

ast_enum!(ElementChild {
    Text(TextNode),
    Element(Element),
    VoidElement(VoidElement),
    Expression(Expression),
});
impl From<ElementExpression> for ElementChild {
    fn from(value: ElementExpression) -> Self {
        match value {
            ElementExpression::Element(e) => e.into(),
            ElementExpression::Void(v) => v.into(),
        }
    }
}
impl<'a> PushNodes<'a> for ElementChild {
    fn push_nodes(&'a self, stack: &mut Vec<crate::Node<'a>>) {
        match self {
            ElementChild::Text(_) => {}
            ElementChild::Element(e) => e.push_children(stack),
            ElementChild::VoidElement(v) => v.push_children(stack),
            ElementChild::Expression(e) => e.push_nodes(stack),
        }
    }
}
