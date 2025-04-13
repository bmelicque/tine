#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Program(Vec<Node>),
    VariableDeclaration {
        name: Option<String>,
        initializer: Option<Box<Node>>,
    },
    ExpressionStatement(Box<Node>),
    BinaryExpression {
        left: Option<Box<Node>>,
        operator: String,
        right: Option<Box<Node>>,
    },
    Identifier(String),
    StringLiteral(String),
    NumberLiteral(f64),
    BooleanLiteral(bool),
    ReturnStatement(Option<Box<Node>>),
}
