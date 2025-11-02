use std::path::PathBuf;

use swc_common::FileName;

use crate::ast::{self, UseTree};

#[derive(Debug, PartialEq)]
pub struct ModuleImports {
    /// The FileName corresponding the module form which names are imported.
    pub module_name: FileName,
    /// One or more trees of imported name from within the declared module.
    pub import_tree: Vec<ast::UseTree>,
}

/// Extract all the `FileName`s use in a `UseDeclaration`
pub fn use_decl_to_paths(base_path: &FileName, decl: &ast::UseDeclaration) -> Vec<ModuleImports> {
    if decl.relative_count == 0 {
        use_virtual_module_imports(decl)
    } else {
        use_real_module_imports(base_path, decl)
    }
}

fn use_virtual_module_imports(decl: &ast::UseDeclaration) -> Vec<ModuleImports> {
    let module_name = FileName::Custom(decl.tree.path[0].as_str().to_string());
    let import_tree = match decl.tree.path.get(1) {
        Some(_) => vec![UseTree {
            path: decl.tree.path.iter().skip(1).cloned().collect(),
            sub_trees: decl.tree.sub_trees.clone(),
        }],
        None => decl.tree.sub_trees.clone(),
    };
    vec![ModuleImports {
        module_name,
        import_tree,
    }]
}

fn use_real_module_imports(base_path: &FileName, decl: &ast::UseDeclaration) -> Vec<ModuleImports> {
    let FileName::Real(mut base) = base_path.clone() else {
        panic!("expected real file name")
    };
    for _ in 0..decl.relative_count {
        base = base.parent().unwrap().to_path_buf();
    }
    use_tree_to_paths(&base, &decl.tree)
}

fn use_tree_to_paths(base: &PathBuf, tree: &ast::UseTree) -> Vec<ModuleImports> {
    let mut path = base.clone();
    for (i, path_element) in tree.path.iter().enumerate() {
        let extended = path.join(path_element.as_str());
        if !extended.exists() {
            return avorted_tree_imports(path, tree, i);
        }
        path = extended
    }
    let mut module_imports: Vec<_> = tree
        .sub_trees
        .iter()
        .flat_map(|sub_tree| use_tree_to_paths(&path, &sub_tree))
        .collect();
    module_imports.sort_by_key(|f| f.module_name.clone());
    module_imports.dedup();
    module_imports
}

fn avorted_tree_imports(path: PathBuf, tree: &ast::UseTree, index: usize) -> Vec<ModuleImports> {
    let sub_tree = UseTree {
        path: tree.path.iter().skip(index).cloned().collect(),
        sub_trees: tree.sub_trees.clone(),
    };
    let imports = ModuleImports {
        module_name: FileName::Real(path),
        import_tree: vec![sub_tree],
    };
    vec![imports]
}
