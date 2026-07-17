// tine_ir/src/macros.rs
#[macro_export]
macro_rules! ir_enum {
    (@typed
        $(#[$meta:meta])*
        $name:ident { $($variant:ident($inner:ty)),* $(,)? }
    ) => {
        $crate::ir_enum!(@emit $(#[$meta])* pub enum $name { $($variant($inner)),* });

        impl Typed for $name {
            fn ty(&self) -> ::tine_types::types::TypeId {
                match self {
                    $(Self::$variant(inner) => Typed::ty(inner),)*
                }
            }
        }
    };

    (
        $(#[$meta:meta])*
        $name:ident { $($variant:ident($inner:ty)),* $(,)? }
    ) => {
        $crate::ir_enum!(@emit $(#[$meta])* pub enum $name { $($variant($inner)),* });
    };

    (@emit
        $(#[$meta:meta])*
        pub enum $name:ident { $($variant:ident($inner:ty)),* $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone)]
        pub enum $name {
            $($variant($inner)),*
        }

        $(
            impl From<$inner> for $name {
                fn from(value: $inner) -> $name {
                    $name::$variant(value)
                }
            }
        )*

        impl Locatable for $name {
            fn loc(&self) -> Location {
                match self {
                    $(Self::$variant(inner) => Locatable::loc(inner),)*
                }
            }
        }

        impl<'a> $name {
            pub fn push_children(&'a self, stack: &mut Vec<$crate::Node<'a>>) {
                match self {
                    $(Self::$variant(inner) => inner.push_children(stack),)*
                }
            }

            pub fn walk(&self) -> $crate::Walk<'_> {
                $crate::Walk { stack: vec![self.into()] } // or Node::Stmt for Statement
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
