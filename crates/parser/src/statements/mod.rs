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
use tine_common::locations::{maybe_vec_loc, Location};

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
            return Some(ast::Statement::Comment(ast::Comment {
                loc: docs.unwrap().loc,
            }));
        };
        match token {
            Token::At => self.parse_meta(docs),
            Token::Break => Some(self.parse_break_statement().into()),
            Token::Continue => Some(self.parse_continue_statement().into()),
            Token::Enum => Some(self.parse_enum(docs, None, None).into()),
            Token::Fn => Some(self.parse_function_definition(docs, None).into()),
            Token::Impl => Some(self.parse_implementations().into()),
            Token::Let => Some(self.parse_variable_declaration(docs, None).into()),
            Token::Pub => self.parse_pub(docs),
            Token::Return => Some(self.parse_return_statement().into()),
            Token::Struct => Some(self.parse_struct_definition(docs, None, None).into()),
            Token::Type => Some(self.parse_type_alias(docs, None).into()),
            _ => self.parse_assignment(),
        }
    }

    pub fn parse_comment(&mut self) -> ast::Comment {
        let mut loc = None;
        while let Some((Ok(Token::LineComment(_)), _)) = self.tokens.peek() {
            let range = self.tokens.next().unwrap().1;
            let chunk = self.localize(range);
            loc = loc.map_or(Some(chunk), |prev| Some(Location::merge(prev, chunk)));
        }
        ast::Comment { loc: loc.unwrap() }
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

        match token {
            Token::Enum => Some(self.parse_enum(docs, None, Some(start)).into()),
            Token::Fn => Some(self.parse_function_definition(docs, Some(start)).into()),
            Token::Let => Some(self.parse_variable_declaration(docs, Some(start)).into()),
            Token::Struct => Some(self.parse_struct_definition(docs, None, Some(start)).into()),
            Token::Type => Some(self.parse_type_alias(docs, Some(start)).into()),
            _ => unreachable!(),
        }
    }

    fn parse_meta(&mut self, docs: Option<ast::Docs>) -> Option<ast::Statement> {
        let mut meta = Vec::new();
        while let Some(attr) = self.maybe_parse_meta_attribute() {
            meta.push(attr);
        }

        let start = self.eat(&[Token::Pub]);
        let start = self.localize(start);

        let tokens = [Token::Enum, Token::Struct];

        let token = match self.tokens.peek() {
            Some((Ok(tok), r)) if tokens.contains(tok) => tok.to_owned(),
            _ => {
                self.recover_before(&tokens, &[Token::Newline]);
                return None;
            }
        };

        match token {
            Token::Enum => Some(self.parse_enum(docs, Some(meta), Some(start)).into()),
            Token::Struct => Some(
                self.parse_struct_definition(docs, Some(meta), Some(start))
                    .into(),
            ),
            _ => unreachable!(),
        }
    }

    fn maybe_parse_meta_attribute(&mut self) -> Option<ast::MetaAttribute> {
        let (_, start_range) = self.maybe_eat(|t| matches!(t, Token::At).then_some(()))?;
        let name = self.maybe_parse_name(|t| matches!(t, Token::LParen | Token::Newline));
        let (args, end_range) = self.maybe_parse_meta_arguments().unzip();

        let loc = maybe_vec_loc(&[Some(start_range), name.as_ref().map(|n| n.loc), end_range])?;
        Some(ast::MetaAttribute { loc, name, args })
    }
    fn maybe_parse_meta_arguments(&mut self) -> Option<(Vec<ast::Identifier>, Location)> {
        self.maybe_eat(|t| matches!(t, Token::LParen).then_some(()))?;
        let args = self.parse_list(
            |self_| {
                let (text, loc) = self_.maybe_eat(|t| t.identifier().map(|s| s.to_owned()))?;
                Some(ast::Identifier { text, loc })
            },
            Token::Comma,
            Token::RParen,
        );
        let end_range = match self.tokens.peek() {
            Some((Ok(Token::RParen), r)) => r.clone(),
            _ => self.recover_at(&[Token::RParen]),
        };
        Some((args, self.localize(end_range)))
    }
}
