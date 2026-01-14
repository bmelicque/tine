use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, ParserEngine},
};

impl ParserEngine {
    pub fn parse_block(&mut self, pair: Pair<'_, Rule>) -> ast::BlockExpression {
        assert_eq!(pair.as_rule(), Rule::block);
        let loc = self.localize(pair.as_span());
        let statements = pair
            .into_inner()
            .map(|pair| self.parse_statement(pair))
            .filter(|stmt| !stmt.is_empty())
            .collect();

        ast::BlockExpression { loc, statements }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{TineParser, Rule};
    use pest::Parser;

    fn parse_expression_input(input: &'static str) -> ast::Expression {
        let pair = TineParser::parse(Rule::expression, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new(0);
        parser_engine.parse_expression(pair)
    }

    #[test]
    fn test_parse_block_expression() {
        let input = r#"{
            x := 42
            x = 43
        }
        "#;
        let result = parse_expression_input(input);

        let ast::Expression::Block(block) = result else {
            panic!("Expected BlockStatement, got {:?}", result)
        };

        assert_eq!(block.statements.len(), 2, "{:?}", block.statements);

        // Check the first statement
        let ast::Statement::VariableDeclaration(var_decl) = &block.statements[0] else {
            panic!("Expected VariableDeclaration")
        };
        match *var_decl.pattern {
            ast::Pattern::Identifier(ast::IdentifierPattern(ref ident))
                if ident.as_str() == "x" => {}
            _ => panic!("Identifier pattern expected"),
        };
        match *var_decl.value.clone() {
            ast::Expression::NumberLiteral(literal) => {
                assert_eq!(literal.value, 42.0)
            }
            _ => panic!("Expected NumberLiteral as variable value"),
        }

        // Check the second statement
        let ast::Statement::Assignment(assignment) = &block.statements[1] else {
            panic!("Expected Assignment")
        };
        match &assignment.pattern {
            ast::Assignee::Pattern(ast::Pattern::Identifier(id)) if id.as_str() == "x" => {}
            _ => panic!("Expected 'x'"),
        }
        match &assignment.value {
            ast::Expression::NumberLiteral(literal) => {
                assert_eq!(literal.value, 43.0)
            }
            _ => panic!("Expected NumberLiteral as assignment value"),
        }
    }
}
