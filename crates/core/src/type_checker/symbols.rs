use enum_from_derive::EnumFrom;

use crate::{type_checker::substitutions::SubstitutionTable, types::TypeId, Location};

#[derive(Clone, Debug)]
pub enum TypeSymbolBody {
    Struct(Vec<(String, MemberSymbolId)>),
    Tuple(Vec<MemberSymbolId>),
}
impl Default for TypeSymbolBody {
    fn default() -> Self {
        Self::Tuple(Vec::new())
    }
}

#[derive(Clone, Debug, Default)]
pub enum MethodReceiverKind {
    #[default]
    Static,
    Immutable,
    Mutable,
}
impl MethodReceiverKind {
    pub fn is_static(&self) -> bool {
        matches!(self, MethodReceiverKind::Static)
    }

    pub fn is_mutable(&self) -> bool {
        matches!(self, MethodReceiverKind::Mutable)
    }
}

#[derive(Copy, Clone, Debug, EnumFrom, PartialEq, Eq, Hash)]
pub enum SymbolId {
    Variable(VariableSymbolId),
    Function(FunctionSymbolId),
    Struct(StructSymbolId),
    Enum(EnumSymbolId),
    Variant(VariantSymbolId),
    Primitive(PrimitiveTypeSymbolId),
    TypeAlias(TypeAliasSymbolId),
    Method(MethodSymbolId),
    Member(MemberSymbolId),
}

impl SymbolId {
    pub fn as_type_symbol_id(self) -> Option<TypeSymbolId> {
        match self {
            SymbolId::Enum(s) => Some(s.into()),
            SymbolId::Struct(s) => Some(s.into()),
            _ => None,
        }
    }

    pub fn dummy() -> Self {
        Self::Variable(VariableSymbolId(usize::MAX))
    }
}

macro_rules! impl_as_id {
    ($ty:ident, $variant:ident, $name:ident) => {
        impl SymbolId {
            pub fn $name(self) -> Option<$ty> {
                match self {
                    Self::$variant(s) => Some(s),
                    _ => None,
                }
            }
        }
    };
}

#[derive(Copy, Clone, Debug, EnumFrom, PartialEq, Eq, Hash)]
pub enum TypeSymbolId {
    Struct(StructSymbolId),
    Enum(EnumSymbolId),
    Primitive(PrimitiveTypeSymbolId),
}
impl Default for TypeSymbolId {
    fn default() -> Self {
        Self::Struct(StructSymbolId::default())
    }
}
impl From<TypeSymbolId> for SymbolId {
    fn from(value: TypeSymbolId) -> Self {
        match value {
            TypeSymbolId::Enum(t) => t.into(),
            TypeSymbolId::Struct(t) => t.into(),
            TypeSymbolId::Primitive(t) => t.into(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct VariableSymbolId(usize);
impl_as_id!(VariableSymbolId, Variable, as_variable);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FunctionSymbolId(usize);
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct StructSymbolId(usize);
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct EnumSymbolId(usize);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct VariantSymbolId(usize);
impl_as_id!(VariantSymbolId, Variant, as_variant);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PrimitiveTypeSymbolId(usize);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeAliasSymbolId(usize);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MethodSymbolId(usize);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MemberSymbolId(usize);
impl_as_id!(MemberSymbolId, Member, as_member);

#[derive(Debug, EnumFrom, Clone)]
pub enum TypeSymbol {
    Enum(EnumSymbol),
    Struct(StructSymbol),
}

#[derive(Debug, Default, Clone)]
pub struct VariableSymbol {
    pub name: String,
    pub mutable: bool,
    pub ty: TypeId,
    pub docs: Option<String>,
    pub defined_at: Location,
    pub access: SymbolAccessManager,
    pub dependencies: Vec<VariableSymbolId>,
}

/// A function name.
/// Its type_id should refer to a FunctionType.
#[derive(Clone, Debug, Default)]
pub struct FunctionSymbol {
    pub name: String,
    // This is expected to have the same length as the function type's params.
    pub param_names: Vec<String>,
    pub ty: TypeId,
    pub docs: Option<String>,
    pub defined_at: Location,
    pub access: SymbolAccessManager,
}

#[derive(Clone, Debug, Default)]
pub struct PrimitiveTypeSymbol {
    pub name: String,
    pub methods: Vec<MethodSymbolId>,
    pub ty: TypeId,
    pub docs: Option<String>,
    pub defined_at: Location,
    pub access: SymbolAccessManager,
}

#[derive(Clone, Debug, Default)]
pub struct TypeAliasSymbol {
    pub name: String,
    pub ty: TypeId,
    pub docs: Option<String>,
    pub defined_at: Location,
    pub access: SymbolAccessManager,
}

#[derive(Clone, Debug, Default)]
pub struct StructSymbol {
    pub name: String,
    pub body: TypeSymbolBody,
    pub methods: Vec<MethodSymbolId>,
    pub ty: TypeId,
    pub docs: Option<String>,
    pub defined_at: Location,
    pub access: SymbolAccessManager,
}

#[derive(Clone, Debug, Default)]
pub struct EnumSymbol {
    pub name: String,
    /// All the variants of the enum.
    /// This should only contain `Constructor` symbols
    pub variants: Vec<VariantSymbolId>,
    pub methods: Vec<MethodSymbolId>,
    pub ty: TypeId,
    pub docs: Option<String>,
    pub defined_at: Location,
    pub access: SymbolAccessManager,
}

/// An enum constructor. In this case, the symbol's `def` should refer to either a `StructType`, a `TupleType` or a `TypeTemplate` wrapping either
#[derive(Clone, Debug, Default)]
pub struct VariantSymbol {
    pub name: String,
    /// The enum symbol owning this.
    pub owner: EnumSymbolId,
    pub body: Option<TypeSymbolBody>,
    pub ty: TypeId,
    pub docs: Option<String>,
    pub defined_at: Location,
    pub access: SymbolAccessManager,
}

#[derive(Clone, Debug, Default)]
pub struct MemberSymbol {
    pub name: String,
    /// The type definition of the struct owning this member.
    pub owner: TypeSymbolId,
    pub ty: TypeId,
    pub docs: Option<String>,
    pub defined_at: Location,
    pub access: SymbolAccessManager,
}

#[derive(Clone, Debug, Default)]
pub struct MethodSymbol {
    pub name: String,
    /// The type definition of the type owning this.
    pub owner: TypeSymbolId,
    /// All the type arguments on the receiver.
    ///
    /// eg in `Type<Arg1, Arg2>.staticMethod()`
    /// (same for instance methods)
    pub owner_args: SubstitutionTable,
    pub receiver: MethodReceiverKind,
    // This is expected to have the same length as the function type's params.
    pub param_names: Vec<String>,
    pub ty: TypeId,
    pub docs: Option<String>,
    pub defined_at: Location,
    pub access: SymbolAccessManager,
}
impl MethodSymbol {
    pub fn concreteness(&self) -> usize {
        self.owner_args.len()
    }

    pub fn matches_substitutions(&self, substitutions: &SubstitutionTable) -> bool {
        self.owner_args == *substitutions
    }

    pub fn is_mutating(&self) -> bool {
        self.receiver.is_mutable()
    }

    pub fn is_static(&self) -> bool {
        self.receiver.is_static()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SymbolAccessManager {
    reads: Vec<Location>,
    writes: Vec<Location>,
    references: Vec<Location>,
}

impl SymbolAccessManager {
    pub fn new() -> Self {
        Self {
            reads: vec![],
            writes: vec![],
            references: vec![],
        }
    }

    pub fn uses(&self) -> impl Iterator<Item = Location> + '_ {
        let reads = self.reads.iter().cloned();
        let writes = self.writes.iter().cloned();
        let refs = self.references.iter().cloned();
        reads.chain(writes).chain(refs)
    }

    pub fn read(&mut self, at: Location) {
        self.reads.push(at);
    }

    pub fn write(&mut self, at: Location) {
        self.writes.push(at);
    }

    pub fn read_to_write(&mut self, at: Location) {
        self.reads.retain(|w| *w != at);
        self.write(at);
    }
}

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

pub trait Symbol {
    fn docs(&self) -> Option<&String>;
    fn name(&self) -> &str;
    fn ty(&self) -> TypeId;
    fn defined_at(&self) -> Location;
    fn uses(&self) -> Box<dyn Iterator<Item = Location> + '_>;
}
pub trait SymbolMut {
    fn access(&mut self) -> &mut SymbolAccessManager;
}
macro_rules! impl_symbol {
    ($symbol:ident) => {
        impl Symbol for $symbol {
            fn docs(&self) -> Option<&String> {
                self.docs.as_ref()
            }

            fn name(&self) -> &str {
                &self.name
            }

            fn ty(&self) -> TypeId {
                self.ty
            }

            fn defined_at(&self) -> Location {
                self.defined_at
            }

            fn uses(&self) -> Box<dyn Iterator<Item = Location> + '_> {
                Box::new(self.access.uses())
            }
        }

        impl SymbolMut for $symbol {
            fn access(&mut self) -> &mut SymbolAccessManager {
                &mut self.access
            }
        }
    };
}
impl_symbol!(VariableSymbol);
impl_symbol!(FunctionSymbol);
impl_symbol!(StructSymbol);
impl_symbol!(EnumSymbol);
impl_symbol!(VariantSymbol);
impl_symbol!(PrimitiveTypeSymbol);
impl_symbol!(TypeAliasSymbol);
impl_symbol!(MethodSymbol);
impl_symbol!(MemberSymbol);
