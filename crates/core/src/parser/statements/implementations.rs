use crate::{
    ast,
    parser::{tokens::Token, Parser},
    DiagnosticKind, Location,
};

impl Parser<'_> {
    pub fn parse_implementations(&mut self) -> ast::Implementation {
        let start_range = self.eat(&[Token::Impl]);
        let start_loc = self.localize(start_range);

        let implemented_type = match self.tokens.peek() {
            Some((Ok(Token::Ident(_)), _)) => Some(self.parse_named_type()),
            _ => None,
        };
        if implemented_type.is_none() {
            let loc = self.next_loc();
            self.error(DiagnosticKind::MissingName, loc);
        }

        let body = self.parse_implementation_body();
        let loc = match (&implemented_type, &body) {
            (_, Some(b)) => Location::merge(start_loc, b.loc),
            (Some(t), None) => Location::merge(start_loc, t.loc),
            _ => start_loc,
        };

        ast::Implementation {
            loc,
            implemented_type,
            body,
        }
    }

    fn parse_implementation_body(&mut self) -> Option<ast::ImplementationBody> {
        let Some((Ok(Token::LBrace), _)) = self.tokens.peek() else {
            let loc = self.next_loc();
            self.error(DiagnosticKind::MissingBody, loc);
            return None;
        };
        let start_range = self.eat(&[Token::LBrace]);
        let start_loc = self.localize(start_range);

        let items = self.parse_list(
            |p| p.parse_implementation_item(),
            Token::Newline,
            Token::RBrace,
        );

        let end_loc = self.close(Token::RBrace);

        Some(ast::ImplementationBody {
            loc: Location::merge(start_loc, end_loc),
            items,
        })
    }

    fn parse_implementation_item(&mut self) -> Option<ast::ImplementationItem> {
        let docs = match self.tokens.peek() {
            Some((Ok(Token::LineComment(_)), range)) => {
                let start = range.start.clone();
                Some(self.parse_docs(start))
            }
            Some((Err(_), _)) | None => return None,
            _ => None,
        };

        let Some((Ok(Token::Fn), _)) = self.tokens.peek() else {
            return None;
        };
        let start_range = self.eat(&[Token::Fn]);
        let start_loc = self.localize(start_range);

        let receiver = match self.tokens.peek() {
            Some((Ok(Token::LParen), _)) => Some(self.parse_method_receiver()),
            _ => None,
        };

        let function = self.parse_function_expression_without_kw();

        match receiver {
            Some(receiver) => {
                let loc = match &function {
                    Some(f) => Location::merge(start_loc, f.loc),
                    None => Location::merge(start_loc, receiver.loc),
                };
                let mut method = ast::MethodDefinition {
                    docs,
                    loc,
                    ..Default::default()
                };
                method.copy_function(function.unwrap_or(ast::FunctionExpression {
                    ..Default::default()
                }));
                Some(ast::ImplementationItem::Method(method))
            }
            None => {
                let loc = function
                    .as_ref()
                    .map_or(start_loc, |f| Location::merge(start_loc, f.loc));
                let mut definition = function.unwrap_or(ast::FunctionExpression::default());
                definition.loc = loc;
                Some(ast::ImplementationItem::StaticMethod(
                    ast::FunctionDefinition { docs, definition },
                ))
            }
        }
    }

    fn parse_method_receiver(&mut self) -> ast::MethodReceiver {
        let start_range = self.eat(&[Token::LParen]);
        let start_loc = self.localize(start_range);

        let pattern = self.parse_pattern();
        let end_range = match self.tokens.peek() {
            Some((Ok(Token::RParen), r)) => r.clone(),
            _ => self.recover_at(&[Token::RParen]),
        };
        let loc = Location::merge(start_loc, self.localize(end_range));
        ast::MethodReceiver { loc, pattern }
    }
}
