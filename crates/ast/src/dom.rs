use tine_common::locations::{Locatable, Location};

use crate::{
    nodes::{ast_enum, ast_struct},
    StringLiteral,
};

use super::Expression;

ast_enum!(ElementExpression {
    Element(Element),
    Void(VoidElement),
});

ast_struct!(Element {
    tag_name: String,
    attributes: Vec<Attribute>,
    children: Vec<ElementChild>,
});
impl Into<Expression> for Element {
    fn into(self) -> Expression {
        Expression::Element(self.into())
    }
}

ast_struct!(VoidElement {
    tag_name: String,
    attributes: Vec<Attribute>,
});
impl Into<Expression> for VoidElement {
    fn into(self) -> Expression {
        Expression::Element(self.into())
    }
}

ast_struct!(TextNode { text: String });

ast_struct!(Attribute {
    name: String,
    value: Option<AttributeValue>,
});

ast_enum!(AttributeValue {
    Expression(Expression),
    String(StringLiteral),
});

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
