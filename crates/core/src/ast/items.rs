use pest::Span;

use crate::{
    ast::{InvalidStatement, Statement},
    parser::utils::merge_span,
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
    pub span: Span<'static>,
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
    pub span: Span<'static>,
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
    pub fn as_span(&self) -> Span<'static> {
        let start = self.span_start();
        let end = self.end();
        merge_span(start, end)
    }

    /// Find the span of the first element of the tree
    fn span_start(&self) -> Span<'static> {
        if let Some(element) = self.path.get(0) {
            return element.span;
        }
        if let Some(subtree) = self.sub_trees.get(0) {
            return subtree.span_start();
        }
        panic!("cannot get span of empty UseTree")
    }

    fn end(&self) -> Span<'static> {
        if let Some(last) = self.sub_trees.last() {
            return last.end();
        }
        if let Some(last) = self.path.last() {
            return last.span;
        }
        panic!("cannot get span of empty UseTree")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathElement {
    pub span: Span<'static>,
}

impl PathElement {
    pub fn as_str(&self) -> &'static str {
        self.span.as_str()
    }
}
