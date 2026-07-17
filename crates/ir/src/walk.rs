use crate::{Block, Expression, Node, Statement};

pub trait PushNodes<'a> {
    fn push_nodes(&'a self, stack: &mut Vec<Node<'a>>);
}

impl<'a> PushNodes<'a> for Expression {
    fn push_nodes(&'a self, stack: &mut Vec<Node<'a>>) {
        self.push_children(stack);
    }
}
impl<'a> PushNodes<'a> for Statement {
    fn push_nodes(&'a self, stack: &mut Vec<Node<'a>>) {
        self.push_children(stack);
    }
}
impl<'a> PushNodes<'a> for Block {
    fn push_nodes(&'a self, stack: &mut Vec<Node<'a>>) {
        self.statements
            .iter()
            .for_each(|stmt| stmt.push_nodes(stack));
    }
}

impl<'a, T: PushNodes<'a>> PushNodes<'a> for Box<T> {
    fn push_nodes(&'a self, stack: &mut Vec<Node<'a>>) {
        (**self).push_nodes(stack);
    }
}
impl<'a, T: PushNodes<'a>> PushNodes<'a> for Vec<T> {
    fn push_nodes(&'a self, stack: &mut Vec<Node<'a>>) {
        for x in self {
            x.push_nodes(stack);
        }
    }
}
impl<'a, T: PushNodes<'a>> PushNodes<'a> for Option<T> {
    fn push_nodes(&'a self, stack: &mut Vec<Node<'a>>) {
        if let Some(x) = self {
            x.push_nodes(stack);
        }
    }
}

pub trait Walkable<'a>: Sized + 'a {
    fn push_children(&'a self, stack: &mut Vec<&'a Self>);
}

pub struct Walk<'a> {
    pub(crate) stack: Vec<Node<'a>>,
}

impl<'a> Iterator for Walk<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Node<'a>> {
        let node = self.stack.pop()?;
        node.push_children(&mut self.stack);
        Some(node)
    }
}
