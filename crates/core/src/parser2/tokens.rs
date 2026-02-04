use logos::Logos;
use ordered_float::OrderedFloat;

#[derive(Logos, Debug, Clone, PartialEq)]
pub enum Token {
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
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,

    // --- punctuation ---
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
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
            Token::Int(i) => i.to_string(),
            Token::Float(f) => f.src.clone(),
            Token::Bool(b) => b.to_string(),
            Token::String(s) => format!("\"{}\"", s),
            Token::Ident(i) => i.clone(),
            Token::Plus => "+".to_string(),
            Token::Minus => "-".to_string(),
            Token::Star => "*".to_string(),
            Token::Slash => "/".to_string(),
            Token::LParen => "(".to_string(),
            Token::RParen => ")".to_string(),
            Token::Comma => ",".to_string(),
            Token::Dot => ".".to_string(),
            Token::FatArrow => "=>".to_string(),
            Token::Whitespace => " ".to_string(),
            Token::Newline => "\n".to_string(),
        }
    }
}
