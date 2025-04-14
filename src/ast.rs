#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: pest::Span<'static>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Program(Vec<AstNode>),
    VariableDeclaration {
        name: Option<String>,
        initializer: Option<Box<AstNode>>,
    },
    Assignment {
        name: Option<String>,
        value: Option<Box<AstNode>>,
    },
    ExpressionStatement(Box<AstNode>),
    BinaryExpression {
        left: Option<Box<AstNode>>,
        operator: String,
        right: Option<Box<AstNode>>,
    },
    Identifier(String),
    StringLiteral(String),
    NumberLiteral(f64),
    BooleanLiteral(bool),
    ReturnStatement(Option<Box<AstNode>>),
}

pub type AstNode = Spanned<Node>;
