use crate::{
    analyzer::ModuleId,
    ast::{self, UseTree},
    common::{use_decl_to_paths, ModuleImports},
    type_checker::TypeChecker,
    DiagnosticKind,
};

impl TypeChecker<'_> {
    pub fn visit_item(&mut self, node: &ast::Item) {
        match node {
            ast::Item::Invalid(_) => {}
            ast::Item::Statement(s) => {
                self.visit_statement(s);
            }
            ast::Item::UseDeclaration(u) => self.visit_use_declaration(u),
        }
    }

    fn visit_use_declaration(&mut self, node: &ast::UseDeclaration) {
        if node.relative_count == 0 {
            self.visit_use_virtual_module(node);
        } else {
            self.visit_use_real_modules(node);
        }
    }

    fn visit_use_virtual_module(&mut self, node: &ast::UseDeclaration) {
        assert_eq!(node.relative_count, 0);
        let module_name = node.tree.path[0].as_str();
        let Some(module_id) = self.session.get_module_id(module_name.into()) else {
            let error = DiagnosticKind::CannotFindModule {
                name: module_name.to_string(),
            };
            self.error(error, node.loc);
            return;
        };
        let subtree = ast::UseTree {
            path: node.tree.path.iter().skip(1).cloned().collect(),
            sub_trees: node.tree.sub_trees.clone(),
        };
        if subtree.path.len() > 0 {
            self.visit_imported_name(module_id, &subtree);
        } else {
            for tree in &subtree.sub_trees {
                self.visit_imported_name(module_id, tree);
            }
        }
    }

    fn visit_use_real_modules(&mut self, node: &ast::UseDeclaration) {
        assert_eq!(node.relative_count, 0);
        let base_path = self.get_file_name();
        let imports = use_decl_to_paths(&base_path, node);
        for import in imports {
            self.visit_module_imports(import);
        }
    }

    /// Visit all the import trees from a given real module.
    ///
    /// For example, `use Module.(a, b.c)` will visit import trees `a` and `b.c` from module `Module`
    fn visit_module_imports(&mut self, imports: ModuleImports) {
        let module_name = imports.module_name;
        let Some(module_id) = self.session.get_module_id(module_name.clone()) else {
            panic!("Cannot find module '{}' within parsed modules", module_name)
        };
        for subtree in &imports.import_tree {
            self.visit_imported_name(module_id, subtree);
        }
    }

    /// Visit an imported element.
    ///
    /// Subvalue imports (like `use Module.value.subvalue`) are not permitted (yet?).
    fn visit_imported_name(&mut self, module: ModuleId, tree: &UseTree) {
        let path_element = &tree.path[0];
        let name = path_element.as_str();
        match self.session.find_export(module, name) {
            Some(symbol) => {
                let ty = symbol.borrow().get_type();
                symbol.borrow().access.read(path_element.loc());
                self.ctx.import(symbol);
                self.ctx.save_expression_type(path_element.loc(), ty);
            }
            None => self.error(
                DiagnosticKind::UnknownMember {
                    member: name.to_string(),
                },
                path_element.loc(),
            ),
        };
        if tree.path.len() > 1 || tree.sub_trees.len() > 0 {
            self.error(DiagnosticKind::UnexpectedModuleTree, tree.loc());
        }
    }
}
