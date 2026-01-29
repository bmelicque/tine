use super::TypeChecker;
use crate::{
    ast,
    diagnostics::DiagnosticKind,
    type_checker::{
        analysis_context::{type_store::TypeStore, SymbolData},
        patterns::TokenList,
    },
    types::{Type, TypeId},
    SymbolKind,
};

impl TypeChecker<'_> {
    pub fn visit_statement(&mut self, node: &ast::Statement) -> TypeId {
        match node {
            ast::Statement::Assignment(node) => self.visit_assignment(node),
            ast::Statement::Break(node) => self.visit_break_statement(node),
            ast::Statement::Empty => TypeStore::UNIT,
            ast::Statement::Enum(node) => self.visit_enum_definition(node),
            ast::Statement::Expression(node) => self.visit_expression(&node.expression),
            ast::Statement::Function(node) => self.visit_function_definition(node),
            ast::Statement::Invalid(_) => TypeStore::UNKNOWN,
            ast::Statement::MethodDefinition(node) => self.visit_method_definition(node),
            ast::Statement::Return(node) => self.visit_return_statement(node),
            ast::Statement::StructDefinition(node) => self.visit_struct_definition(node),
            ast::Statement::TypeAlias(node) => self.visit_type_alias(node),
            ast::Statement::VariableDeclaration(node) => self.visit_variable_declaration(node),
        }
    }

    fn visit_assignment(&mut self, node: &ast::Assignment) -> TypeId {
        let value_type = match &node.value {
            ast::Expression::Empty => TypeStore::UNKNOWN,
            value => self.visit_expression(value),
        };
        self.visit_assignee(&node.pattern, value_type);

        TypeStore::UNIT
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
            let Some(info) = self.lookup_mut(name.as_str()) else {
                self.error(
                    DiagnosticKind::CannotFindName {
                        name: name.as_str().to_string(),
                    },
                    pattern.loc(),
                );
                continue;
            };
            info.write(name.loc);
            self.check_assigned_type(info.borrow().get_type(), ty, pattern.loc());
            if !info.borrow().is_mutable() {
                let error = DiagnosticKind::AssignmentToConstant {
                    name: name.as_str().to_string(),
                };
                self.error(error, pattern.loc());
            }
        }
    }

    fn visit_expr_assignee(&mut self, expr: &ast::MemberExpression, against: TypeId) {
        let ty = self.visit_expression(&expr.object);
        self.check_assigned_type(against, ty, expr.loc);
        let root = expr.root_expression();
        let ast::Expression::Identifier(root) = root else {
            self.error(DiagnosticKind::InvalidRootAssignee, root.loc());
            return;
        };
        let Some(info) = self.lookup_mut(root.as_str()) else {
            let error = DiagnosticKind::CannotFindName {
                name: root.as_str().to_string(),
            };
            self.error(error, root.loc);
            return;
        };
        // visit expression at the beginning of the current scope adds a read
        // so we need to remove it here
        info.read_to_write(root.loc);
        if !info.borrow().is_mutable() {
            let error = DiagnosticKind::AssignmentToConstant {
                name: info.borrow().name.clone(),
            };
            self.error(error, expr.loc);
        }
    }

    fn visit_indirect_assignee(&mut self, node: &ast::IndirectionAssignee, against: TypeId) {
        let name = node.identifier.as_str();
        let Some(info) = self.lookup_mut(&name) else {
            let error = DiagnosticKind::CannotFindName {
                name: name.to_string(),
            };
            self.error(error, node.identifier.loc);
            return;
        };
        info.write(node.identifier.loc);
        let ty = info.borrow().get_type();
        match self.resolve(ty).clone() {
            Type::Signal(t) => {
                self.check_assigned_type(t.inner, against, node.loc);
            }
            Type::Listener(t) => {
                self.check_assigned_type(t.inner, against, node.loc);
            }
            _ => {
                let error = DiagnosticKind::NotDereferenceable {
                    type_name: self.session.display_type(ty),
                };
                self.error(error, node.loc);
            }
        }
    }

    fn visit_break_statement(&mut self, node: &ast::BreakStatement) -> TypeId {
        if let Some(ref value) = node.value {
            self.visit_expression(value);
        }
        TypeStore::UNIT
    }

    fn visit_function_definition(&mut self, node: &ast::FunctionDefinition) -> TypeId {
        let ast::FunctionDefinition { docs, definition } = node;
        let docs = docs.as_ref().map(|d| d.text.clone());
        let Some(ref name) = definition.name else {
            panic!("parser phase should've ensured that name is not None")
        };
        let (ty, dependencies) =
            self.with_dependencies(|checker| checker.visit_function_expression(definition));
        match self.ctx.find_in_current_scope(name.as_str()) {
            Some(symbol) => {
                let error = DiagnosticKind::DuplicateIdentifier {
                    name: name.as_str().into(),
                };
                self.error(error, name.loc);
                symbol
            }
            None => self.ctx.register_symbol(SymbolData {
                name: name.as_str().to_string(),
                ty,
                kind: SymbolKind::Function {
                    param_names: definition
                        .params
                        .iter()
                        .map(|param| param.name.text.clone())
                        .collect(),
                },
                docs,
                defined_at: name.loc,
                dependencies: dependencies.clone(),
                ..Default::default()
            }),
        };

        TypeStore::UNIT
    }

    fn visit_method_definition(&mut self, node: &ast::MethodDefinition) -> TypeId {
        let ((receiver, function), _) = self.with_dependencies(|s| s.visit_method_expression(node));
        let method_name = node.name.as_str();

        if receiver == TypeStore::UNKNOWN {
            return TypeStore::UNIT;
        }

        let field_exists = self.session.types().has_property(receiver, method_name);
        if field_exists {
            let error = DiagnosticKind::DuplicateFieldName {
                name: method_name.to_string(),
            };
            self.error(error, node.loc);
        } else {
            self.session
                .types()
                .define_method(receiver, method_name, function);
        }

        TypeStore::UNIT
    }

    fn visit_method_expression(&mut self, node: &ast::MethodDefinition) -> (TypeId, TypeId) {
        self.with_scope(|checker| {
            let receiver = checker.visit_named_type(&node.receiver.ty);
            checker.ctx.register_symbol(SymbolData {
                name: node.receiver.name.as_str().into(),
                ty: receiver,
                kind: SymbolKind::constant(),
                defined_at: node.receiver.loc,
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
        TypeStore::UNIT
    }

    pub fn visit_variable_declaration(&mut self, node: &ast::VariableDeclaration) -> TypeId {
        let (inferred_type, dependencies) =
            self.with_dependencies(|s| s.visit_expression(&node.value));

        let mutable = node.keyword == ast::DeclarationKeyword::Var;
        if node.pattern.is_refutable() {
            self.error(
                DiagnosticKind::IrrefutablePatternExpected,
                node.pattern.loc(),
            );
        }
        let mut variables = TokenList::new();
        let docs = node.docs.clone().map(|d| d.text);
        self.match_pattern(&node.pattern, inferred_type, &mut variables);
        for (id, ty) in variables.0 {
            match self.ctx.find_in_current_scope(id.as_str()) {
                Some(symbol) => {
                    let error = DiagnosticKind::DuplicateIdentifier {
                        name: id.as_str().to_string(),
                    };
                    self.error(error, id.loc);
                    symbol
                }
                None => self.ctx.register_symbol(SymbolData {
                    name: id.as_str().to_string(),
                    ty,
                    kind: SymbolKind::Value { mutable },
                    docs: docs.clone(),
                    defined_at: node.pattern.loc(),
                    dependencies: dependencies.clone(),
                    ..Default::default()
                }),
            };
        }
        TypeStore::UNIT
    }
}
