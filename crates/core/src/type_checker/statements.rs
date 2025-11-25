use super::TypeChecker;
use crate::{
    ast,
    parser::parser::ParseError,
    type_checker::{
        analysis_context::{type_store::TypeStore, SymbolData},
        patterns::TokenList,
        utils::normalize_doc_comment,
    },
    types::{Type, TypeId},
};

impl TypeChecker {
    pub fn visit_statement(&mut self, node: &ast::Statement) -> TypeId {
        match node {
            ast::Statement::Assignment(node) => self.visit_assignment(node),
            ast::Statement::Empty => TypeStore::VOID,
            ast::Statement::Expression(node) => self.visit_expression(&node.expression),
            ast::Statement::Break(node) => self.visit_break_statement(node),
            ast::Statement::Invalid(_) => TypeStore::UNKNOWN,
            ast::Statement::MethodDefinition(node) => self.visit_method_definition(node),
            ast::Statement::Return(node) => self.visit_return_statement(node),
            ast::Statement::TypeAlias(node) => self.visit_type_declaration(node),
            ast::Statement::VariableDeclaration(node) => self.visit_variable_declaration(node),
        }
    }

    fn visit_assignment(&mut self, node: &ast::Assignment) -> TypeId {
        let value_type = match &node.value {
            ast::Expression::Empty => TypeStore::UNKNOWN,
            value => self.visit_expression(value),
        };
        self.visit_assignee(&node.pattern, value_type);

        TypeStore::VOID
    }

    /// Visit an assignee (i.e. the lhs of an assignment)
    fn visit_assignee(&mut self, assignee: &ast::Assignee, against: TypeId) {
        match assignee {
            ast::Assignee::Member(expr) => self.visit_expr_assignee(expr, against),
            ast::Assignee::Indirection(expr) => self.visit_indirect_assignee(expr, against),
            ast::Assignee::Pattern(pat) => self.visit_pattern_assignee(pat, against),
        }
    }

    /// Visit an assignee which is a pattern
    fn visit_pattern_assignee(&mut self, pattern: &ast::Pattern, against: TypeId) {
        let mut variables = TokenList::new();
        self.match_pattern(pattern, against, &mut variables);
        for (name, ty) in variables.0 {
            let Some(info) = self.analysis_context.lookup_mut(name.as_str()) else {
                self.error(
                    format!("Cannot find name '{}'", name.as_str()),
                    pattern.as_span(),
                );
                continue;
            };
            info.add_write();
            self.check_assigned_type(info.borrow().ty, ty, pattern.as_span());
            if !info.borrow().mutable {
                self.error(
                    "Cannot assign to immutable variable".to_string(),
                    pattern.as_span(),
                );
            }
            self.analysis_context
                .save_symbol_token(name, info.readonly());
        }
    }

    fn visit_expr_assignee(&mut self, expr: &ast::MemberExpression, against: TypeId) {
        let ty = self.visit_expression(&expr.object);
        self.check_assigned_type(against, ty, expr.span);
        let root = expr.root_expression();
        let ast::Expression::Identifier(root) = root else {
            self.errors.push(ParseError {
                message: "Expected identifier".to_string(),
                span: root.as_span(),
            });
            return;
        };
        let Some(info) = self.analysis_context.lookup_mut(root.as_str()) else {
            self.errors.push(ParseError {
                message: format!("Cannot find name '{}'", root.as_str()),
                span: root.span,
            });
            return;
        };
        info.add_write();
        // visit expression at the beginning of the current scope adds a read
        // so we need to remove it here
        info.remove_read();
        if !info.borrow().mutable {
            self.errors.push(ParseError {
                message: "Cannot assign to immutable variable".to_string(),
                span: expr.span,
            });
        }
    }

    fn visit_indirect_assignee(&mut self, node: &ast::IndirectionAssignee, against: TypeId) {
        let name = node.identifier.as_str();
        let Some(info) = self.analysis_context.lookup_mut(&name) else {
            self.error(format!("Cannot find name '{}'", name), node.identifier.span);
            return;
        };
        info.add_write();
        let ty = info.borrow().ty.clone();
        match self.resolve(ty).clone() {
            Type::Signal(t) => {
                self.check_assigned_type(t.inner, against, node.span);
            }
            Type::Listener(t) => {
                self.check_assigned_type(t.inner, against, node.span);
            }
            ref ty => {
                self.error(
                    format!("Cannot dereference variable '{}' of type {}", name, ty),
                    node.span,
                );
            }
        }
        self.analysis_context
            .save_symbol_token(node.identifier.span, info.readonly());
    }

    fn visit_break_statement(&mut self, node: &ast::BreakStatement) -> TypeId {
        if let Some(ref value) = node.value {
            self.visit_expression(value);
        }
        TypeStore::VOID
    }

    fn visit_method_definition(&mut self, node: &ast::MethodDefinition) -> TypeId {
        let ((receiver, function), _) = self.with_dependencies(|s| s.visit_method_expression(node));
        let type_name = node.receiver.ty.name.as_str();
        let method_name = node.name.as_str();

        if receiver == TypeStore::UNKNOWN {
            return TypeStore::VOID;
        }

        let field_exists = self
            .analysis_context
            .type_store
            .has_property(receiver, method_name);
        if field_exists {
            self.error(
                format!(
                    "field '{}' already defined on type '{}'",
                    method_name, type_name
                ),
                node.span,
            );
        } else {
            self.analysis_context
                .type_store
                .define_method(receiver, method_name, function);
        }

        TypeStore::VOID
    }

    fn visit_method_expression(&mut self, node: &ast::MethodDefinition) -> (TypeId, TypeId) {
        self.with_scope(node.span, |checker| {
            let receiver = checker.visit_named_type(&node.receiver.ty);
            checker.analysis_context.register_symbol(SymbolData {
                name: node.receiver.name.as_str().into(),
                ty: receiver,
                defined_at: node.receiver.span,
                ..Default::default()
            });
            let function = checker.visit_function_expression(&node.definition);
            (receiver, function)
        })
    }

    fn visit_return_statement(&mut self, node: &ast::ReturnStatement) -> TypeId {
        if let Some(ref value) = node.value {
            self.visit_expression(value);
        }
        TypeStore::VOID
    }

    pub fn visit_variable_declaration(&mut self, node: &ast::VariableDeclaration) -> TypeId {
        let (inferred_type, dependencies) =
            self.with_dependencies(|s| s.visit_expression(&node.value));

        let mutable = node.op == ast::DeclarationOp::Mut;
        if node.pattern.is_refutable() {
            self.error(
                "Irrefutable pattern expected".into(),
                node.pattern.as_span(),
            );
        }
        let mut variables = TokenList::new();
        let docs = node.docs.map(|d| normalize_doc_comment(d.as_str()));
        self.match_pattern(&node.pattern, inferred_type, &mut variables);
        for (id, ty) in variables.0 {
            let symbol = match self.analysis_context.find_in_current_scope(id.as_str()) {
                Some(symbol) => {
                    let message = format!(
                        "variable '{}' already defined in current scope",
                        id.as_str()
                    );
                    self.error(message, id);
                    symbol
                }
                None => self.analysis_context.register_symbol(SymbolData {
                    name: id.as_str().to_string(),
                    ty,
                    docs: docs.clone(),
                    mutable,
                    defined_at: node.pattern.as_span(),
                    dependencies: dependencies.clone(),
                    ..Default::default()
                }),
            };
            self.analysis_context.save_symbol_token(id, symbol);
        }
        TypeStore::VOID
    }
}
