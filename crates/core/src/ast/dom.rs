use crate::Location;

use super::Expression;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ElementExpression {
    Element(Element),
    Void(VoidElement),
}

impl ElementExpression {
    pub fn loc(&self) -> Location {
        match self {
            ElementExpression::Element(e) => e.loc,
            ElementExpression::Void(v) => v.loc,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Element {
    pub loc: Location,
    pub tag_name: String,
    pub attributes: Vec<Attribute>,
    pub children: Vec<ElementChild>,
}

impl Into<ElementExpression> for Element {
    fn into(self) -> ElementExpression {
        ElementExpression::Element(self)
    }
}
impl Into<Expression> for Element {
    fn into(self) -> Expression {
        Expression::Element(self.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VoidElement {
    pub loc: Location,
    pub tag_name: String,
    pub attributes: Vec<Attribute>,
}

impl Into<ElementExpression> for VoidElement {
    fn into(self) -> ElementExpression {
        ElementExpression::Void(self)
    }
}
impl Into<Expression> for VoidElement {
    fn into(self) -> Expression {
        Expression::Element(self.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextNode {
    pub loc: Location,
    pub text: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Attribute {
    pub loc: Location,
    pub name: String,
    pub value: Option<AttributeValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttributeValue {
    Expression(Expression),
    String(String),
}

impl From<String> for AttributeValue {
    fn from(s: String) -> Self {
        AttributeValue::String(s)
    }
}
impl From<Expression> for AttributeValue {
    fn from(e: Expression) -> Self {
        AttributeValue::Expression(e)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ElementChild {
    Element(Element),
    VoidElement(VoidElement),
    Text(TextNode),
    Expression(Expression),
}
impl From<Element> for ElementChild {
    fn from(e: Element) -> Self {
        ElementChild::Element(e)
    }
}
impl From<VoidElement> for ElementChild {
    fn from(v: VoidElement) -> Self {
        ElementChild::VoidElement(v)
    }
}
impl From<ElementExpression> for ElementChild {
    fn from(value: ElementExpression) -> Self {
        match value {
            ElementExpression::Element(e) => e.into(),
            ElementExpression::Void(v) => v.into(),
        }
    }
}
impl From<TextNode> for ElementChild {
    fn from(t: TextNode) -> Self {
        ElementChild::Text(t)
    }
}
impl From<Expression> for ElementChild {
    fn from(e: Expression) -> Self {
        ElementChild::Expression(e)
    }
}
