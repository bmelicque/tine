use logos::Logos;
use ordered_float::OrderedFloat;

#[derive(Logos, Debug, Clone, PartialEq)]
pub enum Token {
    // --- comments ---
    #[regex(r"//[^\n]*", |lex| lex.slice().to_string(), allow_greedy = true)]
    LineComment(String),

    // --- keywords ---
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token("const")]
    Const,
    #[token("else")]
    Else,
    #[token("enum")]
    Enum,
    #[token("fn")]
    Fn,
    #[token("for")]
    For,
    #[token("if")]
    If,
    #[token("in")]
    In,
    #[token("match")]
    Match,
    #[token("return")]
    Return,
    #[token("struct")]
    Struct,
    #[token("trait")]
    Trait,
    #[token("type")]
    Type,
    #[token("use")]
    Use,
    #[token("var")]
    Var,

    // --- literals ---
    #[regex(r"\d+(_\d+)*", |lex| lex.slice().replace("_", "").parse::<i64>().unwrap())]
    Int(i64),
    #[regex(r"(\d+(_\d+)*\.(\d+(_\d+)*)?)|(\.\d+(_\d+)*)", parse_float)]
    Float(Float),
    #[regex(r"(true)|(false)", |lex| lex.slice() == "true")]
    Bool(bool),
    #[regex(r#""([^"\\]|\\.)*""#, parse_string)]
    String(String),

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    // --- operators ---
    #[token("&&")]
    AndAnd,
    #[token("||")]
    PipePipe,
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token("/>")]
    TagClose,
    #[token("</")]
    LtSlash,
    #[token("<=")]
    Le,
    #[token("<")]
    Lt,
    #[token(">=")]
    Ge,
    #[token(">")]
    Gt,
    #[token("&")]
    And,
    #[token("|")]
    Pipe,
    #[token("@")]
    At,
    #[token("!")]
    Bang,
    #[token("$")]
    Dollar,
    #[token("=")]
    Eq,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("**")]
    StarStar,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Mod,
    #[token("?")]
    QMark,

    // --- punctuation ---
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("=>")]
    FatArrow,

    #[regex(r"[ \t\f]+", logos::skip)]
    Whitespace,
    #[regex(r"[\r\n]+")]
    Newline,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Float {
    pub value: OrderedFloat<f64>,
    pub src: String,
}

fn parse_float(lex: &mut logos::Lexer<Token>) -> Float {
    let slice = lex.slice();
    let src = slice.to_string();
    let value = slice
        .replace("_", "")
        .parse::<f64>()
        .map(|x| OrderedFloat::from(x))
        .unwrap();
    Float { value, src }
}

fn parse_string(lex: &mut logos::Lexer<Token>) -> String {
    let slice = lex.slice(); // includes quotes
    let inner = &slice[1..slice.len() - 1];

    let mut result = String::new();
    let mut chars = inner.chars();

    while let Some(c) = chars.next() {
        if c != '\\' {
            result.push(c);
            continue;
        }

        match chars.next() {
            Some('n') => result.push('\n'),
            Some('t') => result.push('\t'),
            Some('r') => result.push('\r'),
            Some('"') => result.push('"'),
            Some('\\') => result.push('\\'),
            Some(other) => result.push(other), // unknown escape → literal
            None => {}
        }
    }

    result
}

impl Token {
    pub fn to_string(&self) -> String {
        match self {
            Token::LineComment(_) => "comment".to_string(),

            Token::Break => "break".to_string(),
            Token::Const => "const".to_string(),
            Token::Continue => "continue".to_string(),
            Token::Else => "else".to_string(),
            Token::Enum => "enum".to_string(),
            Token::Fn => "fn".to_string(),
            Token::For => "for".to_string(),
            Token::If => "if".to_string(),
            Token::In => "in".to_string(),
            Token::Match => "match".to_string(),
            Token::Return => "return".to_string(),
            Token::Struct => "struct".to_string(),
            Token::Trait => "trait".to_string(),
            Token::Type => "type".to_string(),
            Token::Use => "use".to_string(),
            Token::Var => "var".to_string(),

            Token::Int(i) => i.to_string(),
            Token::Float(f) => f.src.clone(),
            Token::Bool(b) => b.to_string(),
            Token::String(s) => format!("\"{}\"", s),
            Token::Ident(i) => i.clone(),

            Token::AndAnd => "&&".to_string(),
            Token::PipePipe => "||".to_string(),
            Token::EqEq => "==".to_string(),
            Token::NotEq => "!=".to_string(),
            Token::TagClose => "/>".to_string(),
            Token::LtSlash => "</".to_string(),
            Token::Le => "<=".to_string(),
            Token::Lt => "<".to_string(),
            Token::Ge => ">=".to_string(),
            Token::Gt => ">".to_string(),
            Token::And => "&".to_string(),
            Token::Pipe => "|".to_string(),
            Token::At => "@".to_string(),
            Token::Bang => "!".to_string(),
            Token::Dollar => "$".to_string(),
            Token::Eq => "=".to_string(),
            Token::Plus => "+".to_string(),
            Token::Minus => "-".to_string(),
            Token::StarStar => "**".to_string(),
            Token::Star => "*".to_string(),
            Token::Slash => "/".to_string(),
            Token::Mod => "%".to_string(),
            Token::QMark => "?".to_string(),

            Token::LParen => "(".to_string(),
            Token::RParen => ")".to_string(),
            Token::LBrace => "{".to_string(),
            Token::RBrace => "}".to_string(),
            Token::LBracket => "[".to_string(),
            Token::RBracket => "]".to_string(),
            Token::Comma => ",".to_string(),
            Token::Dot => ".".to_string(),
            Token::FatArrow => "=>".to_string(),
            Token::Whitespace => " ".to_string(),
            Token::Newline => "\n".to_string(),
        }
    }

    pub fn precedence(&self) -> u8 {
        match self {
            Token::PipePipe => 1,
            Token::AndAnd => 2,
            Token::EqEq | Token::NotEq => 3,
            Token::Ge | Token::Le | Token::Lt | Token::Gt => 4,
            Token::Plus | Token::Minus => 5,
            Token::Star | Token::Slash | Token::Mod => 6,
            Token::StarStar => 7,
            _ => 0,
        }
    }
}
