use crate::symbols::*;

macro_rules! define_symbols {
    (
        $(
            $variant:ident : $symbol:ident, $id:ident, $arena:ident
        );* $(;)?
    ) => {
        #[derive(Debug, Default)]
        pub struct SymbolTable {
            $($arena: Vec<$symbol>,)*
        }

        impl SymbolTable {
            pub fn insert<I: SymbolIndex>(&mut self, symbol: I::SymbolKind) -> I {
                let arena = I::arena_mut(self);
                let index = arena.len();
                arena.push(symbol);
                I::from_index(index)
            }

            pub fn get<I: SymbolIndex>(&self, id: I) -> &I::SymbolKind {
                &I::arena(self)[id.index()]
            }
            pub fn get_mut<I: SymbolIndex>(&mut self, id: I) -> &mut I::SymbolKind {
                &mut I::arena_mut(self)[id.index()]
            }

            pub fn get_symbol(&self, id: SymbolId) -> &dyn Symbol {
                match id {
                    $(SymbolId::$variant(v) => self.get(v),)*
                }
            }
            pub fn get_symbol_mut(&mut self, id: SymbolId) -> &mut dyn SymbolMut {
                match id {
                    $(SymbolId::$variant(v) => self.get_mut(v),)*
                }
            }

            pub fn is_mutable(&self, id: SymbolId) -> bool {
                match id {
                    SymbolId::Variable(v) => self.get(v).mutable,
                    _ => false,
                }
            }

            pub fn is_public(&self, id: SymbolId) -> bool {
                self.get_symbol(id).is_public()
            }

            pub fn find<I: SymbolIndex, F>(&self, mut predicate: F) -> Option<&I::SymbolKind>
            where
                F: FnMut(&I::SymbolKind) -> bool,
            {
                I::arena(self).iter().find(|&s| predicate(s))
            }

            pub fn find_id<I, F>(&self, mut predicate: F) -> Option<I>
            where
                I: SymbolIndex,
                F: FnMut(&I::SymbolKind) -> bool,
            {
                I::arena(self)
                    .iter()
                    .enumerate()
                    .find(|(_, s)| predicate(s))
                    .map(|(i, _)| I::from_index(i))
            }

            pub fn all(&self) -> impl Iterator<Item = (SymbolId, &dyn Symbol)> + '_ {
                std::iter::empty()
                    $(.chain(self.$arena.iter().enumerate().map(|(i, s)| ($id::from_index(i).into(), s as &dyn Symbol))))*
            }

            pub fn all_ids(&self) -> impl Iterator<Item = SymbolId> + '_ {
                std::iter::empty()
                    $(.chain(ids::<$id>(&self.$arena)))*
            }

            pub fn len(&self) -> usize {
                0 $(+ self.$arena.len())*
            }
        }

        $(
            impl SymbolIndex for $id {
                type SymbolKind = $symbol;
                fn arena(table: &SymbolTable) -> &Vec<Self::SymbolKind> {
                    &table.$arena
                }
                fn arena_mut(table: &mut SymbolTable) -> &mut Vec<Self::SymbolKind> {
                    &mut table.$arena
                }
                fn index(&self) -> usize {
                    self.0
                }
                fn from_index(id: usize) -> Self {
                    Self(id)
                }
            }
        )*
    };
}

fn ids<S: SymbolIndex>(symbols: &[S::SymbolKind]) -> impl Iterator<Item = SymbolId> + '_
where
    S: Into<SymbolId>,
{
    symbols
        .iter()
        .enumerate()
        .map(|(id, _)| S::from_index(id).into())
}

pub trait SymbolIndex {
    type SymbolKind: Symbol;
    fn arena(table: &SymbolTable) -> &Vec<Self::SymbolKind>;
    fn arena_mut(table: &mut SymbolTable) -> &mut Vec<Self::SymbolKind>;
    fn index(&self) -> usize;
    fn from_index(id: usize) -> Self;
}

define_symbols! {
    Variable:  VariableSymbol,       VariableSymbolId,      variables;
    Function:  FunctionSymbol,       FunctionSymbolId,      functions;
    Struct:    StructSymbol,         StructSymbolId,        structs;
    Enum:      EnumSymbol,           EnumSymbolId,          enums;
    Variant:   VariantSymbol,        VariantSymbolId,       variants;
    Primitive: PrimitiveTypeSymbol,  PrimitiveTypeSymbolId, primitives;
    TypeAlias: TypeAliasSymbol,      TypeAliasSymbolId,     aliases;
    Member:    MemberSymbol,         MemberSymbolId,        members;
    Method:    MethodSymbol,         MethodSymbolId,        methods;
    Trait:     TraitSymbol,          TraitSymbolId,         traits;
}
