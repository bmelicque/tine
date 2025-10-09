use std::{cell::RefCell, path::PathBuf, rc::Rc};

use swc_common::FileName;

use crate::{ast, bundler::Module, utils::pretty_print_error};

/// Extract all the `FileName`s use in a `UseDeclaration`
pub fn use_decl_to_paths(file_name: &FileName, decl: &ast::UseDeclaration) -> Vec<FileName> {
    if decl.relative_count == 0 {
        vec![FileName::Custom(decl.tree.path[0].as_str().to_string())]
    } else {
        let FileName::Real(mut base) = file_name.clone() else {
            panic!("expected real file name")
        };
        for _ in 0..decl.relative_count {
            base = base.parent().unwrap().to_path_buf();
        }
        use_tree_to_paths(&base, &decl.tree)
    }
}

fn use_tree_to_paths(base: &PathBuf, tree: &ast::UseTree) -> Vec<FileName> {
    let mut path = base.clone();
    let mut avorted = false;
    for path_element in &tree.path {
        let extended = path.join(path_element.as_str());
        if extended.exists() {
            path = extended
        } else {
            avorted = true;
            break;
        }
    }
    if avorted {
        return vec![path.into()];
    }
    let mut file_names: Vec<FileName> = tree
        .sub_trees
        .iter()
        .flat_map(|sub_tree| use_tree_to_paths(&path, &sub_tree))
        .collect();
    file_names.sort();
    file_names.dedup();
    file_names
}

/// Pretty print all errors found in iterated modules.
///
/// Errors should've been generated during parsing/checking steps.
pub fn print_errors<'a, I>(modules: I) -> bool
where
    I: IntoIterator<Item = &'a Rc<RefCell<Module>>>,
{
    let mut has_errors = false;
    for module in modules {
        for e in &module.borrow().errors {
            has_errors = true;
            pretty_print_error(&e);
        }
    }
    has_errors
}
