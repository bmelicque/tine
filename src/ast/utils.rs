use crate::ast::{
    Alternate, BlockExpression, BreakStatement, Expression, Identifier, IfDeclExpression,
    IfExpression, Loop, PathExpression, ReturnStatement, Statement,
};

fn find_breaks(expr: Expression, stmts: &mut Vec<BreakStatement>) {
    match expr {
        Expression::Block(expr) => expr.find_breaks(stmts),
        Expression::If(expr) => expr.find_breaks(stmts),
        Expression::IfDecl(expr) => expr.find_breaks(stmts),
        Expression::Loop(expr) => expr.find_breaks(stmts),
        _ => (),
    }
}

impl BlockExpression {
    pub fn find_breaks(&self, stmts: &mut Vec<BreakStatement>) {
        for statement in self.statements.iter() {
            match statement {
                Statement::Break(stmt) => stmts.push(stmt.clone()),
                Statement::Expression(ref expr) => find_breaks(*expr.expression.clone(), stmts),
                _ => (),
            }
        }
    }
}

impl IfExpression {
    pub fn find_breaks(&self, stmts: &mut Vec<BreakStatement>) {
        self.consequent.find_breaks(stmts);
        if let Some(ref alternate) = self.alternate {
            alternate.find_breaks(stmts);
        }
    }
}

impl IfDeclExpression {
    pub fn find_breaks(&self, stmts: &mut Vec<BreakStatement>) {
        self.consequent.find_breaks(stmts);
        if let Some(ref alternate) = self.alternate {
            alternate.find_breaks(stmts);
        }
    }
}

impl Alternate {
    pub fn find_breaks(&self, stmts: &mut Vec<BreakStatement>) {
        match self {
            Alternate::Block(expr) => expr.find_breaks(stmts),
            Alternate::If(expr) => expr.find_breaks(stmts),
            Alternate::IfDecl(expr) => expr.find_breaks(stmts),
        }
    }
}

impl Loop {
    pub fn find_breaks(&self, stmts: &mut Vec<BreakStatement>) {
        match self {
            Loop::For(expr) => expr.body.find_breaks(stmts),
            Loop::ForIn(expr) => expr.body.find_breaks(stmts),
        }
    }
}

pub fn find_returns(expr: Expression, stmts: &mut Vec<ReturnStatement>) {
    match expr {
        Expression::Block(expr) => expr.find_returns(stmts),
        Expression::If(expr) => expr.find_returns(stmts),
        Expression::IfDecl(expr) => expr.find_returns(stmts),
        Expression::Loop(expr) => expr.find_returns(stmts),
        _ => (),
    }
}

impl BlockExpression {
    pub fn find_returns(&self, stmts: &mut Vec<ReturnStatement>) {
        for statement in self.statements.iter() {
            match statement {
                Statement::Return(stmt) => stmts.push(stmt.clone()),
                Statement::Expression(ref expr) => find_returns(*expr.expression.clone(), stmts),
                _ => (),
            }
        }
    }
}

impl IfExpression {
    pub fn find_returns(&self, stmts: &mut Vec<ReturnStatement>) {
        self.consequent.find_returns(stmts);
        if let Some(ref alternate) = self.alternate {
            alternate.find_returns(stmts);
        }
    }
}

impl IfDeclExpression {
    pub fn find_returns(&self, stmts: &mut Vec<ReturnStatement>) {
        self.consequent.find_returns(stmts);
        if let Some(ref alternate) = self.alternate {
            alternate.find_returns(stmts);
        }
    }
}

impl Alternate {
    pub fn find_returns(&self, stmts: &mut Vec<ReturnStatement>) {
        match self {
            Alternate::Block(expr) => expr.find_returns(stmts),
            Alternate::If(expr) => expr.find_returns(stmts),
            Alternate::IfDecl(expr) => expr.find_returns(stmts),
        }
    }
}

impl Loop {
    pub fn find_returns(&self, stmts: &mut Vec<ReturnStatement>) {
        match self {
            Loop::For(expr) => expr.body.find_returns(stmts),
            Loop::ForIn(expr) => expr.body.find_returns(stmts),
        }
    }
}

/**
 * Returns the root identifier in an expression, if any.
 * For example, in the expression `a.b.c`, it will return `a`.
 * If expression is just an identifier, it will return that identifier.
 * */
pub fn root_identifier(expr: &Expression) -> Option<Identifier> {
    let root = match expr {
        Expression::FieldAccess(expr) => expr.root_expression(),
        Expression::TupleIndexing(expr) => expr.root_expression(),
        Expression::Identifier(expr) => return Some(expr.clone()),
        _ => return None,
    };
    match root {
        Expression::Identifier(expr) => Some(expr.clone()),
        _ => None,
    }
}
