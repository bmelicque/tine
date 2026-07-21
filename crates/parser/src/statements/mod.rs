mod aliases;
mod assignments;
mod diverging;
mod enums;
mod functions;
mod implementations;
mod structs;
mod traits;
mod utils;
mod variables;

use tine_ast as ast;

use crate::{tokens::Token, utils::normalize_doc_comment, Parser};

impl Parser<'_> {
    pub fn parse_statement(&mut self) -> Option<ast::Statement> {
        let docs = match self.tokens.peek() {
            Some((Ok(Token::LineComment(_)), range)) => {
                let start = range.start.clone();
                Some(self.parse_docs(start))
            }
            Some((Err(_), _)) | None => return None,
            _ => None,
        };
        let Some((Ok(token), _)) = self.tokens.peek() else {
            // This is unreachable because other cases have been handled just above
            unreachable!()
        };
        match token {
            Token::Break => Some(self.parse_break_statement().into()),
            Token::Continue => Some(self.parse_continue_statement().into()),
            Token::Enum => Some(self.parse_enum(docs, None).into()),
            Token::Fn => Some(self.parse_function_definition(docs, None).into()),
            Token::Impl => Some(self.parse_implementations().into()),
            Token::Let => Some(self.parse_variable_declaration(docs, None).into()),
            Token::Pub => self.parse_pub(docs),
            Token::Return => Some(self.parse_return_statement().into()),
            Token::Struct => Some(self.parse_struct_definition(docs, None).into()),
            Token::Type => Some(self.parse_type_alias(docs, None).into()),
            _ => self.parse_assignment(),
        }
    }

    fn parse_docs(&mut self, start: usize) -> ast::Docs {
        let mut text = String::new();
        let mut end = start;
        while let Some((Ok(Token::LineComment(_)), _)) = self.tokens.peek() {
            let Some((Ok(Token::LineComment(line)), range)) = self.tokens.next() else {
                unreachable!()
            };
            text += &normalize_doc_comment(&line);
            end = range.end;
            self.skip_next_if(&Token::Newline);
        }
        ast::Docs {
            text,
            loc: self.localize(start..end),
        }
    }

    fn parse_pub(&mut self, docs: Option<ast::Docs>) -> Option<ast::Statement> {
        let start = self.eat(&[Token::Pub]);
        let start = self.localize(start);

        let tokens = [
            Token::Enum,
            Token::Fn,
            Token::Let,
            Token::Struct,
            Token::Type,
        ];

        let token = match self.tokens.peek() {
            Some((Ok(tok), r)) if tokens.contains(tok) => tok.to_owned(),
            _ => {
                self.recover_before(&tokens, &[Token::Newline]);
                return None;
            }
        };

        // TODO: peek next token
        match token {
            Token::Enum => Some(self.parse_enum(docs, Some(start)).into()),
            Token::Fn => Some(self.parse_function_definition(docs, Some(start)).into()),
            Token::Let => Some(self.parse_variable_declaration(docs, Some(start)).into()),
            Token::Struct => Some(self.parse_struct_definition(docs, Some(start)).into()),
            Token::Type => Some(self.parse_type_alias(docs, Some(start)).into()),
            _ => unreachable!(),
        }
    }
}
