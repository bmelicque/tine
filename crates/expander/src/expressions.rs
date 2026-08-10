use tine_ast::*;

use crate::expander::Expander;

impl Expander {
    pub fn expand_expression(&mut self, expr: Expression) -> Expression {
        use Expression::*;
        match expr {
            Array(a) => self.expand_array(a),
            Binary(b) => self.expand_binary(b),
            Block(b) => self.expand_block(b).into(),
            Call(c) => self.expand_call(c),
            Element(ElementExpression::Element(e)) => self.expand_element(e).into(),
            Element(ElementExpression::Void(e)) => self.expand_void_element(e).into(),
            Function(f) => self.expand_function(f).into(),
            If(i) => self.expand_if(i).into(),
            IfDecl(i) => self.expand_if_pat(i).into(),
            Loop(tine_ast::Loop::For(f)) => self.expand_for(f).into(),
            Loop(tine_ast::Loop::ForIn(f)) => self.expand_for_in(f).into(),
            Match(m) => self.expand_match(m),
            Member(m) => self.expand_member(m),
            Struct(s) => self.expand_struct_expression(s),
            Tuple(t) => self.expand_tuple(t).into(),
            Unary(u) => self.expand_unary(u),
            no_expand => no_expand,
        }
    }

    fn expand_array(&mut self, mut array: ArrayExpression) -> Expression {
        array.elements = array
            .elements
            .into_iter()
            .map(|e| self.expand_expression(e))
            .collect();
        array.into()
    }

    fn expand_binary(&mut self, mut bin: BinaryExpression) -> Expression {
        bin.left = bin.left.map(|e| self.expand_expression(*e).into());
        bin.right = bin.right.map(|e| self.expand_expression(*e).into());
        bin.into()
    }

    pub fn expand_block(&mut self, mut block: BlockExpression) -> BlockExpression {
        block.statements = block
            .statements
            .into_iter()
            .flat_map(|s| self.expand_statement(s))
            .collect();
        block
    }

    fn expand_call(&mut self, mut call: CallExpression) -> Expression {
        call.callee = call.callee.map(|c| self.expand_expression(*c).into());
        call.args = call
            .args
            .into_iter()
            .map(|e| self.expand_call_argument(e))
            .collect();
        call.into()
    }

    fn expand_call_argument(&mut self, arg: CallArgument) -> CallArgument {
        use CallArgument::*;
        match arg {
            Expression(e) => Expression(self.expand_expression(e).into()),
            Callback(mut c) => {
                c.body = c.body.map(|b| self.expand_expression(*b).into());
                Callback(c)
            }
        }
    }

    fn expand_struct_expression(&mut self, mut node: StructExpression) -> Expression {
        node.fields = node
            .fields
            .into_iter()
            .map(|mut f| {
                f.value = f.value.map(|v| self.expand_expression(v).into());
                f
            })
            .collect();
        node.into()
    }

    fn expand_element(&mut self, mut element: Element) -> Element {
        element.attributes = element
            .attributes
            .into_iter()
            .map(|a| self.expand_element_attribute(a))
            .collect();
        element.children = element
            .children
            .into_iter()
            .map(|c| self.expand_element_child(c))
            .collect();
        element
    }

    fn expand_element_attribute(&mut self, mut attribute: Attribute) -> Attribute {
        attribute.value = match attribute.value {
            Some(AttributeValue::Expression(e)) => Some(self.expand_expression(e).into()),
            a => a,
        };
        attribute
    }

    fn expand_element_child(&mut self, child: ElementChild) -> ElementChild {
        use ElementChild::*;
        match child {
            Expression(e) => Expression(self.expand_expression(e).into()),
            Element(e) => Element(self.expand_element(e)),
            VoidElement(e) => VoidElement(self.expand_void_element(e)),
            other => other,
        }
    }

    pub fn expand_function(&mut self, mut function: FunctionExpression) -> FunctionExpression {
        function.body = function.body.map(|b| self.expand_block(b));
        function
    }

    fn expand_if(&mut self, mut if_expr: IfExpression) -> IfExpression {
        if_expr.condition = if_expr.condition.map(|c| self.expand_expression(*c).into());
        if_expr.consequent = if_expr.consequent.map(|t| self.expand_block(t));
        use Alternate::*;
        if_expr.alternate = if_expr.alternate.map(|a| {
            Box::new(match *a {
                Block(b) => self.expand_block(b).into(),
                If(i) => self.expand_if(i).into(),
                IfDecl(i) => self.expand_if_pat(i).into(),
            })
        });
        if_expr
    }

    fn expand_if_pat(&mut self, mut if_expr: IfPatExpression) -> IfPatExpression {
        if_expr.scrutinee = if_expr.scrutinee.map(|c| self.expand_expression(*c).into());
        if_expr.consequent = if_expr.consequent.map(|t| self.expand_block(t));
        use Alternate::*;
        if_expr.alternate = if_expr.alternate.map(|a| {
            Box::new(match *a {
                Block(b) => self.expand_block(b).into(),
                If(i) => self.expand_if(i).into(),
                IfDecl(i) => self.expand_if_pat(i).into(),
            })
        });
        if_expr
    }

    fn expand_for(&mut self, mut for_expr: ForExpression) -> ForExpression {
        for_expr.condition = for_expr
            .condition
            .map(|i| self.expand_expression(*i).into());
        for_expr.body = for_expr.body.map(|b| self.expand_block(b));
        for_expr
    }

    fn expand_for_in(&mut self, mut for_expr: ForInExpression) -> ForInExpression {
        for_expr.iterable = for_expr.iterable.map(|i| self.expand_expression(*i).into());
        for_expr.body = for_expr.body.map(|b| self.expand_block(b));
        for_expr
    }

    fn expand_match(&mut self, mut match_expr: MatchExpression) -> Expression {
        match_expr.scrutinee = match_expr
            .scrutinee
            .map(|i| self.expand_expression(*i).into());
        match_expr.arms = match_expr
            .arms
            .map(|m| m.into_iter().map(|c| self.expand_match_arm(c)).collect());
        match_expr.into()
    }

    fn expand_match_arm(&mut self, mut arm: MatchArm) -> MatchArm {
        arm.expression = arm.expression.map(|e| self.expand_expression(*e).into());
        arm
    }

    fn expand_member(&mut self, mut member: MemberExpression) -> Expression {
        member.object = member.object.map(|o| self.expand_expression(*o).into());
        member.into()
    }

    fn expand_tuple(&mut self, mut tuple: TupleExpression) -> TupleExpression {
        tuple.elements = tuple
            .elements
            .into_iter()
            .map(|e| self.expand_expression(e).into())
            .collect();
        tuple
    }

    fn expand_unary(&mut self, mut unary: UnaryExpression) -> Expression {
        unary.operand = unary.operand.map(|e| self.expand_expression(*e).into());
        unary.into()
    }

    fn expand_void_element(&mut self, mut element: VoidElement) -> VoidElement {
        element.attributes = element
            .attributes
            .into_iter()
            .map(|a| self.expand_element_attribute(a))
            .collect();
        element
    }
}
