#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: pest::Span<'static>,
}

pub type TypeNode = String;

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: Option<TypeNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Program(Vec<AstNode>),
    VariableDeclaration {
        name: Option<String>,
        op: String,
        initializer: Option<Box<AstNode>>,
    },
    Assignment {
        name: Option<String>,
        value: Option<Box<AstNode>>,
    },
    ExpressionStatement(Box<AstNode>),
    Block(Vec<AstNode>),
    BinaryExpression {
        left: Option<Box<AstNode>>,
        operator: String,
        right: Option<Box<AstNode>>,
    },
    FunctionExpression {
        parameters: Option<Vec<Parameter>>,
        return_type: Option<TypeNode>,
        body: Option<Box<AstNode>>, // either a block or expression
    },
    Identifier(String),
    StringLiteral(String),
    NumberLiteral(f64),
    BooleanLiteral(bool),
    ReturnStatement(Option<Box<AstNode>>),
}

pub type AstNode = Spanned<Node>;
