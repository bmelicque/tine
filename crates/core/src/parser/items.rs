use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, ParserEngine},
};

impl ParserEngine {
    pub fn parse_item(&mut self, pair: Pair<'static, Rule>) -> ast::Item {
        assert_eq!(pair.as_rule(), Rule::item);
        let pair = pair.into_inner().next().unwrap();
        match pair.as_rule() {
            Rule::statement => self.parse_statement(pair).into(),
            Rule::use_declaration => self.parse_use_declaration(pair).into(),
            rule => unreachable!("unexpected rule {:?}", rule),
        }
    }

    fn parse_use_declaration(&mut self, pair: Pair<'static, Rule>) -> ast::UseDeclaration {
        assert_eq!(pair.as_rule(), Rule::use_declaration);
        let span = pair.as_span();
        let mut inner = pair.into_inner();

        let tree = self.parse_use_tree(inner.next_back().unwrap());
        let relative_count = self.parse_relative_count(inner.next());

        ast::UseDeclaration {
            span,
            relative_count,
            tree,
        }
    }

    fn parse_relative_count(&mut self, pair: Option<Pair<'static, Rule>>) -> usize {
        let Some(pair) = pair else { return 0 };
        assert_eq!(pair.as_rule(), Rule::relative_count);
        pair.as_str().len()
    }

    fn parse_use_tree(&mut self, pair: Pair<'static, Rule>) -> ast::UseTree {
        assert_eq!(pair.as_rule(), Rule::use_tree);
        let mut path = Vec::new();
        let mut sub_trees = Vec::new();
        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::file_name => path.push(self.parse_file_name(pair)),
                Rule::use_tree => sub_trees.push(self.parse_use_tree(pair)),
                rule => unreachable!("unexpected rule {:?}", rule),
            }
        }
        ast::UseTree { path, sub_trees }
    }

    fn parse_file_name(&mut self, pair: Pair<'static, Rule>) -> ast::PathElement {
        assert_eq!(pair.as_rule(), Rule::file_name);
        ast::PathElement {
            span: pair.as_span(),
        }
    }
}
