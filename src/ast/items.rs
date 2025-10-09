use pest::Span;

use crate::ast::Statement;

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
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
    pub path: Vec<FileName>,
    pub sub_trees: Vec<UseTree>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileName {
    pub span: Span<'static>,
}

impl FileName {
    pub fn as_str(&self) -> &'static str {
        self.span.as_str()
    }
}
