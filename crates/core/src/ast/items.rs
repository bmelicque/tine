use crate::{
    ast::{Identifier, InvalidStatement, Statement},
    locations::Span,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Invalid(InvalidItem),
    UseDeclaration(UseDeclaration),
    Statement(Statement),
}

impl Item {
    pub fn as_use_declaration_ref(&self) -> Option<&UseDeclaration> {
        match self {
            Item::UseDeclaration(u) => Some(u),
            _ => None,
        }
    }
}

impl From<Statement> for Item {
    fn from(value: Statement) -> Self {
        Self::Statement(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InvalidItem {
    pub span: Span,
}
impl Into<Item> for InvalidItem {
    fn into(self) -> Item {
        Item::Invalid(self)
    }
}
impl From<InvalidStatement> for InvalidItem {
    fn from(value: InvalidStatement) -> Self {
        InvalidItem { span: value.span }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UseDeclaration {
    pub span: Span,
    pub relative_count: usize,
    pub tree: UseTree,
}

impl Into<Item> for UseDeclaration {
    fn into(self) -> Item {
        Item::UseDeclaration(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UseTree {
    pub path: Vec<PathElement>,
    pub sub_trees: Vec<UseTree>,
}

impl UseTree {
    pub fn as_span(&self) -> Span {
        let start = self.span_start();
        let end = self.end();
        Span::merge(start, end)
    }

    /// Find the span of the first element of the tree
    fn span_start(&self) -> Span {
        if let Some(element) = self.path.get(0) {
            return element.0.span;
        }
        if let Some(subtree) = self.sub_trees.get(0) {
            return subtree.span_start();
        }
        panic!("cannot get span of empty UseTree")
    }

    fn end(&self) -> Span {
        if let Some(last) = self.sub_trees.last() {
            return last.end();
        }
        if let Some(last) = self.path.last() {
            return last.0.span;
        }
        panic!("cannot get span of empty UseTree")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathElement(pub Identifier);

impl PathElement {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn as_span(&self) -> Span {
        self.0.span
    }
}
