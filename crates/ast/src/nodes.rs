use crate::walk::*;

pub enum Node<'a> {
    Expr(&'a crate::Expression),
    Stmt(&'a crate::Statement),
}
impl<'a> Node<'a> {
    pub fn as_expression(&self) -> Option<&'a crate::Expression> {
        match self {
            Node::Expr(e) => Some(e),
            _ => None,
        }
    }
    pub fn as_statement(&self) -> Option<&'a crate::Statement> {
        match self {
            Node::Stmt(s) => Some(s),
            _ => None,
        }
    }

    pub(crate) fn push_children(&self, stack: &mut Vec<Node<'a>>) {
        match self {
            Node::Expr(e) => e.push_nodes(stack),
            Node::Stmt(s) => s.push_nodes(stack),
        }
    }
}

macro_rules! ast_enum {
    (
        $name:ident {
            $($variant:ident($inner:ty)),* $(,)?
        }
    ) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum $name {
            $($variant($inner)),*
        }

        $(
            impl From<$inner> for $name {
                fn from(v: $inner) -> Self {
                    Self::$variant(v)
                }
            }
        )*

        impl Locatable for $name {
            fn loc(&self) -> Location {
                match self {
                    $(Self::$variant(x) => x.loc(),)*
                }
            }
        }

        impl<'a> $name {
            pub fn push_children(&'a self, stack: &mut Vec<$crate::Node<'a>>) {
                match self {
                    $(Self::$variant(inner) => inner.push_children(stack),)*
                }
            }

            paste::paste! {
                $(pub fn [<as_ $variant:snake>](&self) -> Option<&$inner> {
                    match self {
                        Self::$variant(inner) => Some(inner),
                        _ => None,
                    }
                })*
            }
        }
    };
}

macro_rules! operator_enum {
    (
        $name:ident {
            $($variant:ident => $text:literal),* $(,)?
        }
    ) => {
        #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            #[default]
            $($variant),*
        }

        impl $name {
            pub fn as_str(&self) -> &str {
                match self {
                    $($name::$variant => $text,)*
                }
            }

            pub fn to_string(&self) -> String {
                self.as_str().to_string()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.as_str())
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                match value.as_str() {
                    $($text => $name::$variant,)*

                    _ => panic!("Invalid operator")
                }
            }
        }
    };
}

pub(crate) use ast_enum;
pub(crate) use operator_enum;
