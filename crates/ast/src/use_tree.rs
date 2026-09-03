use std::path::PathBuf;

use tine_common::{locations::Location, module_path::ModulePath};

use crate::{UseDeclaration, UseTree};

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleIdentifier {
    pub loc: Location,
    pub path: ModulePath,
}
impl ModuleIdentifier {
    pub fn new(path: ModulePath, loc: Location) -> Self {
        Self { path, loc }
    }
}

#[derive(Debug, PartialEq)]
pub struct ModuleImports {
    /// The ModuleIdentifier corresponding the module form which names are imported.
    pub module_name: ModuleIdentifier,
    /// One or more trees of imported name from within the declared module.
    pub import_tree: Vec<UseTree>,
}

/// Extract all the `FileName`s use in a `UseDeclaration`
pub fn use_decl_to_paths(base_path: &ModulePath, decl: &UseDeclaration) -> Vec<ModuleImports> {
    if decl.relative_count == 0 {
        use_virtual_module_imports(decl)
    } else {
        use_real_module_imports(base_path, decl)
    }
}

fn use_virtual_module_imports(decl: &UseDeclaration) -> Vec<ModuleImports> {
    let Some(path_element) = decl.tree.path.get(0) else {
        return vec![];
    };
    let module_path = ModulePath::Virtual(path_element.as_str().to_string());
    let module_name = ModuleIdentifier::new(module_path, path_element.loc());
    let import_tree = match decl.tree.path.get(1) {
        Some(_) => vec![UseTree {
            path: decl.tree.path.iter().skip(1).cloned().collect(),
            sub_trees: decl.tree.sub_trees.clone(),
        }],
        None => decl.tree.sub_trees.clone(),
    };
    vec![ModuleImports {
        module_name: module_name,
        import_tree,
    }]
}

fn use_real_module_imports(base_path: &ModulePath, decl: &UseDeclaration) -> Vec<ModuleImports> {
    let ModulePath::Real(mut base) = base_path.clone() else {
        panic!("expected real file name, got {:?}", base_path)
    };
    for _ in 0..decl.relative_count {
        base = base.parent().unwrap().to_path_buf();
    }
    use_tree_to_paths(&base, &decl.tree)
}

fn use_tree_to_paths(base: &PathBuf, tree: &UseTree) -> Vec<ModuleImports> {
    let mut path = base.clone();
    for (i, path_element) in tree.path.iter().enumerate() {
        let as_dir = path.join(path_element.as_str());
        if as_dir.exists() {
            path = as_dir;
            continue;
        }
        let as_file = path.join(format!("{}.tine", path_element.as_str()));
        if as_file.exists() {
            path = as_file;
            continue;
        }
        return avorted_tree_imports(path, tree, i, path_element.loc());
    }
    let mut module_imports: Vec<_> = tree
        .sub_trees
        .iter()
        .flat_map(|sub_tree| use_tree_to_paths(&path, &sub_tree))
        .collect();
    module_imports.sort_by_key(|f| f.module_name.path.clone());
    module_imports.dedup();
    module_imports
}

fn avorted_tree_imports(
    path: PathBuf,
    tree: &UseTree,
    index: usize,
    loc: Location,
) -> Vec<ModuleImports> {
    let sub_tree = UseTree {
        path: tree.path.iter().skip(index).cloned().collect(),
        sub_trees: tree.sub_trees.clone(),
    };
    let module_name = ModuleIdentifier::new(ModulePath::Real(path), loc);
    let imports = ModuleImports {
        module_name,
        import_tree: vec![sub_tree],
    };
    vec![imports]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_not_crash_when_empty() {
        let item = UseDeclaration::default();
        assert_eq!(use_virtual_module_imports(&item), vec![]);
    }
}
