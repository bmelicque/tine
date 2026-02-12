use enum_from_derive::EnumFrom;

use crate::{
    ast::{Identifier, InvalidStatement, Statement},
    Location,
};

#[derive(Debug, EnumFrom, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct InvalidItem {
    pub loc: Location,
}
impl From<InvalidStatement> for InvalidItem {
    fn from(value: InvalidStatement) -> Self {
        InvalidItem { loc: value.loc }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UseDeclaration {
    pub loc: Location,
    pub relative_count: usize,
    pub tree: UseTree,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UseTree {
    pub path: Vec<PathElement>,
    pub sub_trees: Vec<UseTree>,
}

impl UseTree {
    pub fn loc(&self) -> Location {
        let start = self.span_start();
        let end = self.end();
        Location::merge(start, end)
    }

    /// Find the span of the first element of the tree
    fn span_start(&self) -> Location {
        if let Some(element) = self.path.get(0) {
            return element.0.loc;
        }
        if let Some(subtree) = self.sub_trees.get(0) {
            return subtree.span_start();
        }
        panic!("cannot get span of empty UseTree")
    }

    fn end(&self) -> Location {
        if let Some(last) = self.sub_trees.last() {
            return last.end();
        }
        if let Some(last) = self.path.last() {
            return last.0.loc;
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

    pub fn loc(&self) -> Location {
        self.0.loc
    }
}
