#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: pest::Span<'static>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: Option<Box<AstNode>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SumTypeConstructor {
    pub name: String,
    pub param: Option<AstNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Program(Vec<AstNode>),

    // Types
    UnaryType(Option<Box<AstNode>>), // []Type | ?Type
    Tuple(Vec<Option<AstNode>>),
    BinaryType {
        left: Option<Box<AstNode>>,
        operator: String,
        right: Option<Box<AstNode>>,
    },
    GenericType {
        name: String,
        args: Vec<Box<AstNode>>,
    },

    // Statements
    VariableDeclaration {
        name: Option<String>,
        op: String,
        initializer: Option<Box<AstNode>>,
    },
    Assignment {
        name: Option<String>,
        value: Option<Box<AstNode>>,
    },
    TypeDeclaration {
        name: String,
        def: Option<Box<AstNode>>,
    },
    Struct(Vec<Spanned<(String, Option<Box<AstNode>>)>>),
    Sum(Vec<SumTypeConstructor>),
    Trait {
        name: String,
        body: Box<AstNode>,
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
        return_type: Option<Box<AstNode>>,
        body: Option<Box<AstNode>>, // either a block or expression
    },
    Identifier(String),
    StringLiteral(String),
    NumberLiteral(f64),
    BooleanLiteral(bool),
    ReturnStatement(Option<Box<AstNode>>),
}

pub type AstNode = Spanned<Node>;
