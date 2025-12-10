use crate::{ast, ParseError};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModulePath {
    /// A file path to a file in the project.
    ///
    /// This is expected to be an absolute, canonical path.
    Real(std::path::PathBuf),
    /// The name of another module, usually from the standard library or a
    /// project dependency
    Virtual(String),
}
impl From<std::path::PathBuf> for ModulePath {
    fn from(value: std::path::PathBuf) -> Self {
        Self::Real(value)
    }
}
impl From<&std::path::PathBuf> for ModulePath {
    fn from(value: &std::path::PathBuf) -> Self {
        Self::Real(value.clone())
    }
}
impl From<String> for ModulePath {
    fn from(value: String) -> Self {
        Self::Virtual(value)
    }
}
impl From<&str> for ModulePath {
    fn from(value: &str) -> Self {
        Self::Virtual(value.to_string())
    }
}
impl std::fmt::Display for ModulePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModulePath::Real(p) => write!(f, "{}", p.display()),
            ModulePath::Virtual(c) => write!(f, "{}", c),
        }
    }
}

pub type ModuleId = usize;

#[derive(Debug, Clone)]
pub struct ParsedModule {
    pub(super) id: ModuleId,
    pub name: ModulePath,
    pub ast: ast::Program,
    pub errors: Vec<ParseError>,
}

#[derive(Debug, Clone)]
pub struct ModuleBuilder {
    name: Option<ModulePath>,
    ast: Option<ast::Program>,
    errors: Vec<ParseError>,
}

impl ParsedModule {
    pub(super) fn builder() -> ModuleBuilder {
        ModuleBuilder {
            name: None,
            ast: None,
            errors: vec![],
        }
    }
}

impl ModuleBuilder {
    pub(super) fn name(mut self, path: impl Into<ModulePath>) -> Self {
        self.name = Some(path.into());
        self
    }

    pub(super) fn ast(mut self, program: ast::Program) -> Self {
        self.ast = Some(program);
        self
    }

    pub(super) fn errors(mut self, errors: Vec<ParseError>) -> Self {
        self.errors.extend(errors);
        self
    }

    pub(super) fn build(self) -> ParsedModule {
        ParsedModule {
            id: 0,
            name: self.name.unwrap(),
            ast: self.ast.unwrap_or(ast::Program::dummy()),
            errors: self.errors,
        }
    }
}
