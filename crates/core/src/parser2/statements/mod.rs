mod aliases;
mod assignments;
mod diverging;
mod enums;
mod functions;
mod structs;
mod utils;
mod variables;

use crate::{
    ast,
    parser2::{tokens::Token, utils::normalize_doc_comment, Parser},
};

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
        let statement = match token {
            Token::Break => Some(self.parse_break_statement().into()),
            Token::Const | Token::Var => Some(self.parse_variable_declaration(docs).into()),
            Token::Enum => Some(self.parse_enum(docs).into()),
            Token::Fn => Some(self.parse_function_definition(docs).into()),
            Token::Return => Some(self.parse_return_statement().into()),
            Token::Struct => Some(self.parse_struct_definition(docs).into()),
            Token::Type => Some(self.parse_type_alias(docs).into()),
            _ => self.parse_assignment(),
        };
        self.expect(Token::Newline);

        statement
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
}
