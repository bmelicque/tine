use pest::iterators::Pair;

use crate::ast;

use super::{parser::Rule, ParserEngine};

impl ParserEngine {
    pub fn parse_statement(&mut self, pair: Pair<'static, Rule>) -> ast::Statement {
        match pair.as_rule() {
            Rule::statement => {
                let inner_pair = pair.into_inner().next().unwrap();
                self.parse_statement(inner_pair)
            }
            Rule::variable_declaration => self.parse_variable_declaration(pair).into(),
            Rule::assignment => self.parse_assignment(pair).into(),
            Rule::type_alias => self.parse_type_alias(pair).into(),
            Rule::return_statement => self.parse_return_statement(pair).into(),
            Rule::block => self.parse_block(pair).into(),
            Rule::expression_statement => self.parse_expression_statement(pair).into(),
            _ => ast::Statement::Empty,
        }
    }

    fn parse_variable_declaration(
        &mut self,
        pair: Pair<'static, Rule>,
    ) -> ast::VariableDeclaration {
        let span = pair.as_span();
        let mut inner = pair.into_inner();

        let name = inner.next().unwrap().as_str().to_string();
        let op = inner.next().unwrap().as_str().to_string().into();
        let value = Box::new(self.parse_expression(inner.next().unwrap()));

        ast::VariableDeclaration {
            span,
            name,
            op,
            value,
        }
    }

    fn parse_assignment(&mut self, pair: Pair<'static, Rule>) -> ast::Assignment {
        let span = pair.as_span();
        let mut inner = pair.into_inner();

        let name = inner.next().unwrap().as_str().to_string();
        let value = self.parse_expression(inner.next().unwrap());

        ast::Assignment { span, name, value }
    }

    fn parse_return_statement(&mut self, pair: Pair<'static, Rule>) -> ast::ReturnStatement {
        let span = pair.as_span();
        let value = pair
            .into_inner()
            .next()
            .map(|inner| self.parse_expression(inner))
            .map(Box::new);

        ast::ReturnStatement { span, value }
    }

    pub fn parse_block(&mut self, pair: Pair<'static, Rule>) -> ast::BlockStatement {
        let span = pair.as_span();
        let statements = pair
            .into_inner()
            .map(|pair| self.parse_statement(pair))
            .collect();

        ast::BlockStatement { span, statements }
    }

    fn parse_expression_statement(
        &mut self,
        pair: Pair<'static, Rule>,
    ) -> ast::ExpressionStatement {
        let Some(inner) = pair.into_inner().next() else {
            return ast::Expression::Empty.into();
        };
        self.parse_expression(inner).into()
    }
}
