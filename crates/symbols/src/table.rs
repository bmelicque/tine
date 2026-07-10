use crate::symbols::*;

#[derive(Debug, Default)]
pub struct SymbolTable {
    variables: Vec<VariableSymbol>,
    functions: Vec<FunctionSymbol>,
    structs: Vec<StructSymbol>,
    enums: Vec<EnumSymbol>,
    variants: Vec<VariantSymbol>,
    primitives: Vec<PrimitiveTypeSymbol>,
    aliases: Vec<TypeAliasSymbol>,
    members: Vec<MemberSymbol>,
    methods: Vec<MethodSymbol>,
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
        use SymbolId::*;
        match id {
            Variable(v) => self.get(v),
            Function(f) => self.get(f),
            Struct(s) => self.get(s),
            Enum(e) => self.get(e),
            Variant(v) => self.get(v),
            Primitive(p) => self.get(p),
            TypeAlias(s) => self.get(s),
            Method(m) => self.get(m),
            Member(m) => self.get(m),
        }
    }
    pub fn get_symbol_mut(&mut self, id: SymbolId) -> &mut dyn SymbolMut {
        use SymbolId::*;
        match id {
            Variable(v) => self.get_mut(v),
            Function(f) => self.get_mut(f),
            Struct(s) => self.get_mut(s),
            Enum(e) => self.get_mut(e),
            Variant(v) => self.get_mut(v),
            Primitive(p) => self.get_mut(p),
            TypeAlias(t) => self.get_mut(t),
            Method(m) => self.get_mut(m),
            Member(m) => self.get_mut(m),
        }
    }

    pub fn is_mutable(&self, id: SymbolId) -> bool {
        match id {
            SymbolId::Variable(v) => self.get(v).mutable,
            _ => false,
        }
    }
    pub fn is_public(&self, id: SymbolId) -> bool {
        use SymbolId::*;
        match id {
            Variable(v) => self.get(v).public,
            Function(f) => self.get(f).public,
            Struct(s) => self.get(s).public,
            Enum(e) => self.get(e).public,
            Variant(_) => true,
            Primitive(_) => true,
            TypeAlias(t) => self.get(t).public,
            Method(m) => self.get(m).public,
            Member(m) => self.get(m).public,
        }
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

    pub fn all(&self) -> Vec<&dyn Symbol> {
        let mut symbols: Vec<&dyn Symbol> = Vec::new();

        symbols.extend(self.variables.iter().map(|s| s as &dyn Symbol));
        symbols.extend(self.functions.iter().map(|s| s as &dyn Symbol));
        symbols.extend(self.structs.iter().map(|s| s as &dyn Symbol));
        symbols.extend(self.enums.iter().map(|s| s as &dyn Symbol));
        symbols.extend(self.variants.iter().map(|s| s as &dyn Symbol));
        symbols.extend(self.primitives.iter().map(|s| s as &dyn Symbol));
        symbols.extend(self.aliases.iter().map(|s| s as &dyn Symbol));
        symbols.extend(self.members.iter().map(|s| s as &dyn Symbol));
        symbols.extend(self.methods.iter().map(|s| s as &dyn Symbol));

        symbols
    }

    pub fn all_ids(&self) -> impl Iterator<Item = SymbolId> + '_ {
        ids::<VariableSymbolId>(&self.variables)
            .chain(ids::<FunctionSymbolId>(&self.functions))
            .chain(ids::<StructSymbolId>(&self.structs))
            .chain(ids::<EnumSymbolId>(&self.enums))
            .chain(ids::<VariantSymbolId>(&self.variants))
            .chain(ids::<PrimitiveTypeSymbolId>(&self.primitives))
            .chain(ids::<TypeAliasSymbolId>(&self.aliases))
            .chain(ids::<MemberSymbolId>(&self.members))
            .chain(ids::<MethodSymbolId>(&self.methods))
    }
}
fn ids<S: SymbolIndex>(symbols: &[S::SymbolKind]) -> impl Iterator<Item = SymbolId> + '_
where
    S: Into<SymbolId>,
{
    symbols
        .into_iter()
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

macro_rules! define_symbol_index {
    ($symbol:ident, $id:ident, $arena:ident) => {
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
    };
}

define_symbol_index!(VariableSymbol, VariableSymbolId, variables);
define_symbol_index!(FunctionSymbol, FunctionSymbolId, functions);
define_symbol_index!(StructSymbol, StructSymbolId, structs);
define_symbol_index!(EnumSymbol, EnumSymbolId, enums);
define_symbol_index!(VariantSymbol, VariantSymbolId, variants);
define_symbol_index!(PrimitiveTypeSymbol, PrimitiveTypeSymbolId, primitives);
define_symbol_index!(TypeAliasSymbol, TypeAliasSymbolId, aliases);
define_symbol_index!(MethodSymbol, MethodSymbolId, methods);
define_symbol_index!(MemberSymbol, MemberSymbolId, members);
