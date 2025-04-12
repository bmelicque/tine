#[derive(Debug, Clone)]
pub enum Node {
    Program(Vec<Node>),
    VariableDeclaration {
        name: String,
        type_annotation: Option<String>,
        initializer: Option<Box<Node>>,
    },
    FunctionDeclaration {
        name: String,
        params: Vec<(String, String)>, // (name, type)
        return_type: Option<String>,
        body: Vec<Node>,
    },
    BinaryExpression {
        left: Box<Node>,
        operator: String,
        right: Box<Node>,
    },
    Identifier(String),
    StringLiteral(String),
    NumberLiteral(f64),
    BooleanLiteral(bool),
    ReturnStatement(Option<Box<Node>>),
}
