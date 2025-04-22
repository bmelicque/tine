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
pub struct StructField {
    pub name: String,
    pub def: Option<Box<AstNode>>,
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Program(Vec<AstNode>),

    // Types
    NamedType(String), // for named types like String, Number, etc.
    UnaryType {
        operator: String,
        inner: Option<Box<AstNode>>,
    }, // []Type | ?Type | &Type
    TupleType(Vec<Option<AstNode>>),
    BinaryType {
        left: Option<Box<AstNode>>,
        operator: String,
        right: Option<Box<AstNode>>,
    },
    GenericType {
        name: Box<AstNode>, // should be a Node::Identifier
        args: Vec<Box<AstNode>>,
    },
    FunctionType {
        parameters: Box<AstNode>,
        return_type: Box<AstNode>,
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
        type_params: Option<Vec<String>>,
        def: Option<Box<AstNode>>,
    },
    Struct(Vec<Spanned<StructField>>),
    SumDef(Vec<SumTypeConstructor>),
    TraitDef {
        name: String,
        body: Box<AstNode>,
    },
    ExpressionStatement(Box<AstNode>),
    Block(Vec<AstNode>),
    ReturnStatement(Option<Box<AstNode>>),

    // Expressions
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

    // Instances
    MapLiteral {
        ty: Option<Box<AstNode>>,
        entries: Vec<Spanned<MapEntry>>,
    },
    UnaryLiteral {
        unary_type: Box<AstNode>,
        body: Vec<AstNode>,
    },
    StructLiteral {
        struct_type: Box<AstNode>,
        fields: Vec<Spanned<FieldAssignment>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapEntry {
    pub key: Box<AstNode>,
    pub value: Box<AstNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldAssignment {
    pub name: String,
    pub value: Box<AstNode>,
}

pub type AstNode = Spanned<Node>;
