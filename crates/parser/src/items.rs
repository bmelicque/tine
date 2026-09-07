use tine_ast::*;
use tine_common::{diagnostics::DiagnosticKind, locations::Location};

use crate::{tokens::Token, Parser};

impl Parser<'_> {
    pub fn parse_item(&mut self) -> Option<Item> {
        match self.tokens.peek() {
            Some((Ok(Token::Use), _)) => {}
            Some((Ok(Token::Newline), _)) => return None,
            _ => return self.parse_statement_item(),
        };

        let start_range = self.eat(&[Token::Use]);
        let start_loc = self.localize(start_range);

        let mut relative_count = 0;
        while let Some((Ok(Token::Dot), _)) = self.tokens.peek() {
            self.eat(&[Token::Dot]);
            relative_count += 1;
        }

        let tree = self.parse_use_tree();
        if tree.is_none() {
            let error_loc = self.next_loc().nth_char(0);
            self.error(DiagnosticKind::MissingName, error_loc);
        }

        Some(Item::UseDeclaration(UseDeclaration {
            loc: Location::merge(start_loc, self.next_loc()),
            relative_count,
            tree: tree.unwrap_or(UseTree {
                path: vec![],
                sub_trees: vec![],
            }),
        }))
    }

    fn parse_use_tree(&mut self) -> Option<UseTree> {
        let mut path = Vec::new();
        match self.tokens.peek() {
            Some((Ok(Token::Ident(_)), _)) => {
                path.push(PathElement(self.parse_identifier()));
            }
            _ => return None,
        }
        while let Some((Ok(Token::Dot), _)) = self.tokens.peek() {
            self.tokens.next(); // consume the dot
            match self.tokens.peek() {
                Some((Ok(Token::Ident(_)), _)) => {
                    path.push(PathElement(self.parse_identifier()));
                }
                Some((Ok(Token::LBrace), _)) => break,
                _ => {
                    let error_loc = self.next_loc();
                    self.error(DiagnosticKind::MissingName, error_loc);
                }
            }
        }
        let sub_trees = match self.tokens.peek() {
            Some((Ok(Token::LBrace), _)) => {
                self.eat(&[Token::LBrace]);
                let sub_trees =
                    self.parse_list(|p| p.parse_use_tree(), Token::Comma, Token::RBrace);
                match self.tokens.peek() {
                    Some((Ok(Token::RBrace), _)) => self.eat(&[Token::RBrace]),
                    _ => self.recover_at(&[Token::RBrace]),
                };
                sub_trees
            }
            _ => vec![],
        };
        Some(UseTree { path, sub_trees })
    }

    fn parse_statement_item(&mut self) -> Option<Item> {
        let stmt = self.parse_statement()?;
        match &stmt {
            Statement::Function(f) => {
                self.validate_function_definition(f);
            }
            _ => {}
        }
        Some(stmt.into())
    }
}
