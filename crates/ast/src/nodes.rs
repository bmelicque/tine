macro_rules! ast_struct {
    (
        $(#[$meta:meta])*
        $name:ident {
            $($field:ident : $ty:ty),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name {
            pub loc: Location,
            $(pub $field: $ty),*
        }

        impl Locatable for $name {
            fn loc(&self) -> Location {
                self.loc
            }
        }
    };
}

macro_rules! ast_enum {
    (
        $name:ident {
            $($variant:ident($inner:ty)),* $(,)?
        }
    ) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
pub(crate) use ast_struct;
pub(crate) use operator_enum;
