use crate::{
    ast,
    common::{use_decl_to_paths, ModuleImports},
    type_checker::{self, dom_metadata, ModuleMetadata, TypeChecker},
};

impl TypeChecker {
    pub fn visit_item(&mut self, node: &ast::Item) {
        match node {
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
        let metadata = match module_name {
            "dom" => dom_metadata(),
            name => panic!("Unexpected virtual module '{}'", name),
        };
        let subtree = ast::UseTree {
            path: node.tree.path.iter().skip(1).cloned().collect(),
            sub_trees: node.tree.sub_trees.clone(),
        };
        if subtree.path.len() > 0 {
            self.visit_imported_name(&subtree, &metadata);
        } else {
            for tree in subtree.sub_trees {
                self.visit_imported_name(&tree, &metadata);
            }
        }
    }

    fn visit_use_real_modules(&mut self, node: &ast::UseDeclaration) {
        assert_eq!(node.relative_count, 0);
        let base_path = self.get_file_name().unwrap();
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
        let Some(module) = self.get_module(&module_name) else {
            panic!("Cannot find module '{}' within parsed modules", module_name)
        };
        // cloning rc to stop borrowing self
        let module = module.clone();
        let module = module.borrow();
        let Some(ref metadata) = module.context else {
            panic!(
                "Module '{}' should've already been type-checked at this point!",
                module_name
            )
        };

        for subtree in &imports.import_tree {
            self.visit_imported_name(&subtree, metadata);
        }
    }

    /// Visit an imported element.
    ///
    /// Subvalue imports (like `use Module.value.subvalue`) are not permitted (yet?).
    fn visit_imported_name(&mut self, tree: &ast::UseTree, metadata: &ModuleMetadata) {
        let path_element = &tree.path[0];
        let name = path_element.as_str();
        let symbol = metadata.exports.values().find(|s| s.name.as_str() == name);
        match symbol {
            Some(symbol) => {
                self.analysis_context
                    .register_symbol(type_checker::Symbol::pure(
                        name.to_string(),
                        symbol.ty.clone(),
                        path_element.span,
                    ));
                self.set_type_at(path_element.span, symbol.ty.clone());
            }
            None => self.error(
                format!("This module has no exported element named '{}'", name),
                path_element.span,
            ),
        };
        if tree.path.len() > 1 || tree.sub_trees.len() > 0 {
            self.error(
                "Cannot import subvalues (not implemented yet)".to_string(),
                tree.as_span(),
            );
        }
    }
}
