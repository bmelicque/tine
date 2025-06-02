use crate::ast::{
    Alternate, BlockExpression, BreakStatement, Expression, IfDeclExpression, IfExpression, Loop,
    Statement,
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
