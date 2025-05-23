use pest::iterators::Pair;

use crate::{ast, parser::utils::merge_span};

use super::{
    parser::{ParseError, Rule},
    ParserEngine,
};

impl ParserEngine {
    pub fn parse_expression(&mut self, pair: Pair<'static, Rule>) -> ast::Expression {
        match pair.as_rule() {
            Rule::anonymous_expression
            | Rule::expression
            | Rule::primary
            | Rule::tuple_or_expression
            | Rule::type_annotation => {
                if let Some(inner) = pair.into_inner().next() {
                    self.parse_expression(inner)
                } else {
                    ast::Expression::Empty
                }
            }
            Rule::array_expression => self.parse_array_expression(pair).into(),
            Rule::block => self.parse_block(pair).into(),
            Rule::composite_literal => self.parse_composite_literal(pair).into(),
            Rule::equality | Rule::relation | Rule::addition | Rule::multiplication => {
                self.parse_binary_ltr_expression(pair).into()
            }
            Rule::exponentiation => self.parse_exponentiation(pair).into(),
            Rule::value_identifier => self.parse_identifier(pair).into(),
            Rule::if_expression => self.parse_if_expression(pair).into(),
            Rule::member_expression => self.parse_field_access_expression(pair).into(),
            Rule::tuple_expression => self.parse_tuple_expression(pair).into(),
            Rule::tuple_indexing => self.parse_tuple_indexing(pair).into(),
            Rule::string_literal => ast::StringLiteral {
                span: pair.as_span(),
            }
            .into(),
            Rule::number_literal => self.parse_number_literal(pair).into(),
            Rule::boolean_literal => ast::BooleanLiteral {
                span: pair.as_span(),
                value: pair.as_str() == "true",
            }
            .into(),
            _ => ast::Expression::Empty,
        }
    }

    pub fn parse_expression_or_anonymous(
        &mut self,
        pair: Pair<'static, Rule>,
    ) -> ast::ExpressionOrAnonymous {
        // { struct_literal_body | array_literal_body | expression }
        assert!(pair.as_rule() == Rule::anonymous_expression);
        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::expression => self.parse_expression(inner).into(),
            Rule::struct_literal_body => self.parse_anonymous_struct_literal(inner).into(),
            _ => panic!(),
        }
    }

    fn parse_array_expression(&mut self, pair: Pair<'static, Rule>) -> ast::ArrayExpression {
        let span = pair.as_span();
        let elements = pair
            .into_inner()
            .map(|element| self.parse_expression(element))
            .collect();
        ast::ArrayExpression { span, elements }
    }

    fn parse_binary_ltr_expression(&mut self, pair: Pair<'static, Rule>) -> ast::Expression {
        let span = pair.as_span().to_owned();
        let mut inner = pair.into_inner();
        let Some(next) = inner.next() else {
            return ast::Expression::Empty;
        };
        let mut left = self.parse_expression(next);

        let mut is_binary = false;
        while let Some(op_pair) = inner.next() {
            if !is_binary && left.is_empty() {
                self.errors.push(ParseError {
                    message: "Expression expected".to_string(),
                    span: op_pair.as_span(),
                });
            }
            is_binary = true;
            let operator = op_pair.as_str().to_string();

            let Some(right_pair) = inner.next() else {
                self.errors.push(ParseError {
                    message: "Expression expected".to_string(),
                    span: op_pair.as_span(),
                });
                continue;
            };

            let right = self.parse_expression(right_pair);
            if right.is_empty() {
                self.errors.push(ParseError {
                    message: "Expression expected".to_string(),
                    span: op_pair.as_span(),
                });
            }

            left = ast::BinaryExpression {
                span,
                left: Box::new(left),
                operator: operator.into(),
                right: Box::new(right),
            }
            .into();
        }

        left
    }

    fn parse_block(&mut self, pair: Pair<'static, Rule>) -> ast::BlockExpression {
        let span = pair.as_span();
        let statements = pair
            .into_inner()
            .map(|pair| self.parse_statement(pair))
            .filter(|stmt| !stmt.is_empty())
            .collect();

        ast::BlockExpression { span, statements }
    }

    fn parse_exponentiation(&mut self, pair: Pair<'static, Rule>) -> ast::Expression {
        assert!(pair.as_rule() == Rule::exponentiation);
        let span = pair.as_span();
        let mut node = ast::Expression::Empty;
        for sub_pair in pair.into_inner().rev() {
            let left = self.parse_expression(sub_pair);
            if node == ast::Expression::Empty {
                node = left;
                continue;
            }
            node = ast::BinaryExpression {
                left: Box::new(left),
                operator: ast::BinaryOperator::Pow,
                right: Box::new(node),
                // FIXME:
                span,
            }
            .into();
        }
        node
    }

    fn parse_field_access_expression(
        &mut self,
        pair: Pair<'static, Rule>,
    ) -> ast::FieldAccessExpression {
        assert!(pair.as_rule() == Rule::member_expression);
        let mut inner = pair.into_inner();
        let mut node = self.parse_expression(inner.next().unwrap());

        while let Some(sub_pair) = inner.next() {
            let right_span = sub_pair.as_span();
            let prop = self.parse_identifier(sub_pair);
            let left_span = node.as_span();
            node = ast::FieldAccessExpression {
                span: merge_span(left_span, right_span),
                object: Box::new(node),
                prop,
            }
            .into()
        }

        match node {
            ast::Expression::FieldAccess(n) => n,
            _ => panic!("Unexpected variant!"),
        }
    }

    fn parse_tuple_expression(&mut self, pair: Pair<'static, Rule>) -> ast::TupleExpression {
        let span = pair.as_span();
        let elements = pair
            .into_inner()
            .map(|pair| self.parse_expression(pair))
            .collect();
        ast::TupleExpression { span, elements }
    }

    fn parse_tuple_indexing(&mut self, pair: Pair<'static, Rule>) -> ast::TupleIndexingExpression {
        assert!(pair.as_rule() == Rule::tuple_indexing);
        let span = pair.as_span();
        let mut inner = pair.into_inner();

        let tuple = Box::new(self.parse_expression(inner.next().unwrap()));

        let index = self.parse_number_literal(inner.next().unwrap());

        ast::TupleIndexingExpression { span, tuple, index }
    }

    fn parse_number_literal(&mut self, pair: Pair<'static, Rule>) -> ast::NumberLiteral {
        ast::NumberLiteral {
            span: pair.as_span(),
            value: pair.as_str().parse().unwrap_or(0.0),
        }
    }

    fn parse_identifier(&mut self, pair: Pair<'static, Rule>) -> ast::Identifier {
        ast::Identifier {
            span: pair.as_span(),
        }
    }

    fn parse_if_expression(&mut self, pair: Pair<'static, Rule>) -> ast::IfExpression {
        assert!(pair.as_rule() == Rule::if_expression);
        let span = pair.as_span();
        let mut inner = pair.into_inner();
        let condition = Box::new(self.parse_expression(inner.next().unwrap()));
        let consequent = Box::new(self.parse_block(inner.next().unwrap()));
        let alternate = inner
            .next()
            .map(|pair| Box::new(self.parse_alternate(pair)));
        ast::IfExpression {
            span,
            condition,
            consequent,
            alternate,
        }
    }

    fn parse_if_decl_expression(&mut self, pair: Pair<'static, Rule>) -> ast::IfDeclExpression {
        assert!(pair.as_rule() == Rule::if_decl_expression);
        let span = pair.as_span();
        let mut inner = pair.into_inner();
        let pattern = Box::new(self.parse_pattern(inner.next().unwrap()));
        let scrutinee = Box::new(self.parse_expression(inner.next().unwrap()));
        let consequent = Box::new(self.parse_block(inner.next().unwrap()));
        let alternate = inner
            .next()
            .map(|pair| Box::new(self.parse_alternate(pair)));
        ast::IfDeclExpression {
            span,
            pattern,
            scrutinee,
            consequent,
            alternate,
        }
    }

    fn parse_alternate(&mut self, pair: Pair<'static, Rule>) -> ast::Alternate {
        assert!(pair.as_rule() == Rule::alternate);
        let pair = pair.into_inner().next().unwrap();
        match pair.as_rule() {
            Rule::block => self.parse_block(pair).into(),
            Rule::if_expression => self.parse_if_expression(pair).into(),
            Rule::if_decl_expression => self.parse_if_decl_expression(pair).into(),
            rule => unreachable!("unexpected rule {:?}", rule),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{MyLanguageParser, Rule};
    use pest::Parser;

    fn parse_expression_input(input: &'static str, rule: Rule) -> ast::Expression {
        let pair = MyLanguageParser::parse(rule, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new();
        parser_engine.parse_expression(pair)
    }

    #[test]
    fn test_parse_identifier() {
        let input = "myVariable";
        let result = parse_expression_input(input, Rule::value_identifier);

        match result {
            ast::Expression::Identifier(identifier) => {
                assert_eq!(identifier.span.as_str(), "myVariable");
            }
            _ => panic!("Expected Identifier"),
        }
    }

    #[test]
    fn test_parse_number_literal() {
        let input = "42";
        let result = parse_expression_input(input, Rule::number_literal);

        match result {
            ast::Expression::NumberLiteral(literal) => {
                assert_eq!(literal.value, 42.0);
            }
            _ => panic!("Expected NumberLiteral"),
        }
    }

    #[test]
    fn test_parse_array_expression() {
        let input = "[1, 2, 3]";
        let pair = MyLanguageParser::parse(Rule::array_expression, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new();
        let result = parser_engine.parse_array_expression(pair);

        assert_eq!(result.elements.len(), 3);

        assert!(matches!(
            result.elements[0],
            ast::Expression::NumberLiteral(ast::NumberLiteral { value, .. }) if value == 1.0
        ));

        assert!(matches!(
            result.elements[1],
            ast::Expression::NumberLiteral(ast::NumberLiteral { value, .. }) if value == 2.0
        ));

        assert!(matches!(
            result.elements[2],
            ast::Expression::NumberLiteral(ast::NumberLiteral { value, .. }) if value == 3.0
        ));
    }

    #[test]
    fn test_parse_binary_expression() {
        let input = "1 + 2 * 3";
        let result = parse_expression_input(input, Rule::expression);

        match result {
            ast::Expression::Binary(binary) => {
                assert_eq!(binary.operator, ast::BinaryOperator::Add);
                match *binary.left {
                    ast::Expression::NumberLiteral(left) => assert_eq!(left.value, 1.0),
                    _ => panic!("Expected NumberLiteral on the left"),
                }
                match *binary.right {
                    ast::Expression::Binary(inner_binary) => {
                        assert_eq!(inner_binary.operator, ast::BinaryOperator::Mul);
                        match *inner_binary.left {
                            ast::Expression::NumberLiteral(left) => assert_eq!(left.value, 2.0),
                            _ => panic!("Expected NumberLiteral on the left"),
                        }
                        match *inner_binary.right {
                            ast::Expression::NumberLiteral(right) => assert_eq!(right.value, 3.0),
                            _ => panic!("Expected NumberLiteral on the right"),
                        }
                    }
                    _ => panic!("Expected BinaryExpression on the right"),
                }
            }
            _ => panic!("Expected BinaryExpression"),
        }
    }

    #[test]
    fn test_parse_block_expression() {
        let input = r#"{
            x := 42
            x = 43
        }
        "#;
        let result = parse_expression_input(input, Rule::expression);

        match result {
            ast::Expression::Block(block) => {
                assert_eq!(block.statements.len(), 2, "{:?}", block.statements);

                // Check the first statement
                match &block.statements[0] {
                    ast::Statement::VariableDeclaration(var_decl) => {
                        match *var_decl.pattern {
                            ast::Pattern::Identifier(ast::IdentifierPattern { span })
                                if span.as_str() == "x" => {}
                            _ => panic!("Identifier pattern expected"),
                        };
                        match *var_decl.value.clone() {
                            ast::Expression::NumberLiteral(literal) => {
                                assert_eq!(literal.value, 42.0)
                            }
                            _ => panic!("Expected NumberLiteral as variable value"),
                        }
                    }
                    _ => panic!("Expected VariableDeclaration"),
                }

                // Check the second statement
                match &block.statements[1] {
                    ast::Statement::Assignment(assignment) => {
                        match &assignment.pattern {
                            ast::PatternExpression::Pattern(ast::Pattern::Identifier(id))
                                if id.span.as_str() == "x" => {}
                            _ => panic!("Expected 'x'"),
                        }
                        match &assignment.value {
                            ast::Expression::NumberLiteral(literal) => {
                                assert_eq!(literal.value, 43.0)
                            }
                            _ => panic!("Expected NumberLiteral as assignment value"),
                        }
                    }
                    _ => panic!("Expected Assignment"),
                }
            }
            _ => panic!("Expected BlockStatement, got {:?}", result),
        }
    }

    #[test]
    fn test_parse_field_access_expression() {
        let input = "object.property";
        let result = parse_expression_input(input, Rule::member_expression);

        match result {
            ast::Expression::FieldAccess(field_access) => {
                assert_eq!(field_access.object.as_span().as_str(), "object");
                assert_eq!(field_access.prop.span.as_str(), "property");
            }
            _ => panic!("Expected FieldAccessExpression"),
        }
    }

    #[test]
    fn test_parse_tuple_indexing_expression() {
        let input = "tuple.0";
        let result = parse_expression_input(input, Rule::tuple_indexing);

        match result {
            ast::Expression::TupleIndexing(tuple_indexing) => {
                assert_eq!(tuple_indexing.tuple.as_span().as_str(), "tuple");
                assert_eq!(tuple_indexing.index.value, 0.0);
            }
            _ => panic!("Expected TupleIndexingExpression"),
        }
    }

    #[test]
    fn test_parse_exponentiation_expression() {
        let input = "2 ** 3 ** 2";
        let result = parse_expression_input(input, Rule::exponentiation);

        match result {
            ast::Expression::Binary(binary) => {
                assert_eq!(binary.operator, ast::BinaryOperator::Pow);
                match *binary.left {
                    ast::Expression::NumberLiteral(left) => assert_eq!(left.value, 2.0),
                    _ => panic!("Expected NumberLiteral on the left"),
                }
                match *binary.right {
                    ast::Expression::Binary(inner_binary) => {
                        assert_eq!(inner_binary.operator, ast::BinaryOperator::Pow);
                        match *inner_binary.left {
                            ast::Expression::NumberLiteral(left) => assert_eq!(left.value, 3.0),
                            _ => panic!("Expected NumberLiteral on the left"),
                        }
                        match *inner_binary.right {
                            ast::Expression::NumberLiteral(right) => assert_eq!(right.value, 2.0),
                            _ => panic!("Expected NumberLiteral on the right"),
                        }
                    }
                    _ => panic!("Expected BinaryExpression on the right"),
                }
            }
            _ => panic!("Expected BinaryExpression"),
        }
    }

    #[test]
    fn test_parse_tuple_expression_with_multiple_elements() {
        let input = "(1, \"hello\", true)";
        let result = parse_expression_input(input, Rule::tuple_or_expression);

        let ast::Expression::Tuple(result) = result else {
            panic!("Tuple expected")
        };
        assert_eq!(result.elements.len(), 3);

        // Check the first element
        assert!(matches!(
            result.elements[0],
            ast::Expression::NumberLiteral(ast::NumberLiteral { value, .. }) if value == 1.0
        ));

        // Check the second element
        assert!(matches!(
            result.elements[1],
            ast::Expression::StringLiteral(ast::StringLiteral { ref span, .. }) if span.as_str() == "\"hello\""
        ));

        // Check the third element
        assert!(matches!(
            result.elements[2],
            ast::Expression::BooleanLiteral(ast::BooleanLiteral { value, .. }) if value == true
        ));
    }

    #[test]
    fn test_parse_nested_tuple_expression() {
        let input = "(1, (\"nested\", false))";
        let result = parse_expression_input(input, Rule::tuple_or_expression);
        let ast::Expression::Tuple(result) = result else {
            panic!("Expected tuple!")
        };

        assert_eq!(result.elements.len(), 2);

        // Check the first element
        assert!(matches!(
            result.elements[0],
            ast::Expression::NumberLiteral(ast::NumberLiteral { value, .. }) if value == 1.0
        ));

        // Check the second element (nested tuple)
        if let ast::Expression::Tuple(nested_tuple) = &result.elements[1] {
            assert_eq!(nested_tuple.elements.len(), 2);

            // Check the first element of the nested tuple
            assert!(matches!(
                nested_tuple.elements[0],
                ast::Expression::StringLiteral(ast::StringLiteral { ref span, .. }) if span.as_str() == "\"nested\""
            ));

            // Check the second element of the nested tuple
            assert!(matches!(
                nested_tuple.elements[1],
                ast::Expression::BooleanLiteral(ast::BooleanLiteral { value, .. }) if value == false
            ));
        } else {
            panic!("Expected a nested tuple");
        }
    }
}
