use crate::ir::{
    Block, BreakStatement, Expression, ForExpression, ForInExpression, Identifier, IfExpression,
    ReturnStatement, Statement,
};

fn find_breaks(expr: &Expression) -> Vec<BreakStatement> {
    match expr {
        Expression::Block(expr) => expr.find_breaks(),
        Expression::For(expr) => expr.find_breaks(),
        Expression::ForIn(expr) => expr.find_breaks(),
        Expression::If(expr) => expr.find_breaks(),
        _ => vec![],
    }
}

impl Block {
    pub fn find_breaks(&self) -> Vec<BreakStatement> {
        let mut stmts = Vec::new();
        for statement in self.statements.iter() {
            match statement {
                Statement::Break(stmt) => stmts.push(stmt.clone()),
                Statement::Expression(expr) => stmts.extend(find_breaks(&expr)),
                _ => (),
            }
        }
        stmts
    }
}

impl IfExpression {
    pub fn find_breaks(&self) -> Vec<BreakStatement> {
        vec![
            self.consequent.find_breaks(),
            self.alternate.as_ref().map_or(vec![], |a| a.find_breaks()),
        ]
        .concat()
    }
}

impl ForExpression {
    pub fn find_breaks(&self) -> Vec<BreakStatement> {
        self.body.find_breaks()
    }
}

impl ForInExpression {
    pub fn find_breaks(&self) -> Vec<BreakStatement> {
        self.body.find_breaks()
    }
}

fn find_returns(expr: &Expression) -> Vec<ReturnStatement> {
    match expr {
        Expression::Block(expr) => expr.find_returns(),
        Expression::For(expr) => expr.find_returns(),
        Expression::ForIn(expr) => expr.find_returns(),
        Expression::If(expr) => expr.find_returns(),
        _ => vec![],
    }
}

impl Block {
    pub fn find_returns(&self) -> Vec<ReturnStatement> {
        let mut stmts = Vec::new();
        for statement in self.statements.iter() {
            match statement {
                Statement::Return(stmt) => stmts.push(stmt.clone()),
                Statement::Expression(expr) => stmts.extend(find_returns(&expr)),
                _ => (),
            }
        }
        stmts
    }
}

impl IfExpression {
    pub fn find_returns(&self) -> Vec<ReturnStatement> {
        vec![
            self.consequent.find_returns(),
            self.alternate.as_ref().map_or(vec![], |a| a.find_returns()),
        ]
        .concat()
    }
}

impl ForExpression {
    pub fn find_returns(&self) -> Vec<ReturnStatement> {
        self.body.find_returns()
    }
}

impl ForInExpression {
    pub fn find_returns(&self) -> Vec<ReturnStatement> {
        self.body.find_returns()
    }
}

/**
 * Returns the root identifier in an expression, if any.
 * For example, in the expression `a.b.c`, it will return `a`.
 * If expression is just an identifier, it will return that identifier.
 * */
pub fn root_identifier(expr: &Expression) -> Option<&Identifier> {
    match expr {
        Expression::Member(expr) => root_identifier(&expr.object),
        Expression::Identifier(expr) => Some(expr),
        _ => None,
    }
}
