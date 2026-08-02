use tine_macros::EnumFrom;

pub type ModuleId = usize;

#[derive(Debug, EnumFrom, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModulePath {
    /// A file path to a file in the project.
    ///
    /// This is expected to be an absolute, canonical path.
    Real(std::path::PathBuf),
    /// The name of another module, usually from the standard library or a
    /// project dependency
    Virtual(String),
}
impl From<&std::path::PathBuf> for ModulePath {
    fn from(value: &std::path::PathBuf) -> Self {
        Self::Real(value.clone())
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
impl Default for ModulePath {
    fn default() -> Self {
        Self::Virtual("".to_string())
    }
}
