use std::collections::HashMap;

use enum_from_derive::EnumFrom;
use tine_common::locations::Location;
use tine_types::types::{TypeId, TypeParam};

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
    Trait(TraitSymbolId),
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
pub struct VariableSymbolId(pub(crate) usize);
impl_as_id!(VariableSymbolId, Variable, as_variable);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FunctionSymbolId(pub(crate) usize);
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct StructSymbolId(pub(crate) usize);
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct EnumSymbolId(pub(crate) usize);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct VariantSymbolId(pub(crate) usize);
impl_as_id!(VariantSymbolId, Variant, as_variant);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PrimitiveTypeSymbolId(pub(crate) usize);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeAliasSymbolId(pub(crate) usize);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MethodSymbolId(pub(crate) usize);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TraitSymbolId(pub(crate) usize);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MemberSymbolId(pub(crate) usize);
impl_as_id!(MemberSymbolId, Member, as_member);

#[derive(Debug, EnumFrom, Clone)]
pub enum TypeSymbol {
    Enum(EnumSymbol),
    Struct(StructSymbol),
}

#[derive(Debug, Default, Clone)]
pub struct VariableSymbol {
    pub name: String,
    pub public: bool,
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
    pub public: bool,
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
    pub public: bool,
    pub methods: Vec<MethodSymbolId>,
    pub ty: TypeId,
    pub docs: Option<String>,
    pub defined_at: Location,
    pub access: SymbolAccessManager,
}

#[derive(Clone, Debug, Default)]
pub struct TypeAliasSymbol {
    pub name: String,
    pub public: bool,
    pub ty: TypeId,
    pub docs: Option<String>,
    pub defined_at: Location,
    pub access: SymbolAccessManager,
}

#[derive(Clone, Debug, Default)]
pub struct StructSymbol {
    pub name: String,
    pub public: bool,
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
    pub public: bool,
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
    pub public: bool,
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
    pub public: bool,
    /// The type definition of the type owning this.
    pub owner: TypeSymbolId,
    /// All the type arguments on the receiver.
    ///
    /// eg in `Type<Arg1, Arg2>.staticMethod()`
    /// (same for instance methods)
    pub owner_args: HashMap<TypeParam, TypeId>,
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

    pub fn matches_substitutions(&self, substitutions: &HashMap<TypeParam, TypeId>) -> bool {
        self.owner_args == *substitutions
    }

    pub fn is_mutating(&self) -> bool {
        self.receiver.is_mutable()
    }

    pub fn is_static(&self) -> bool {
        self.receiver.is_static()
    }
}

#[derive(Clone, Debug, Default)]
pub struct TraitSymbol {
    pub name: String,
    pub public: bool,
    // This is expected to have the same length as the function type's params.
    pub param_names: Vec<String>,
    pub ty: TypeId,
    pub docs: Option<String>,
    pub defined_at: Location,
    pub access: SymbolAccessManager,
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

pub trait Symbol {
    fn docs(&self) -> Option<&String>;
    fn name(&self) -> &str;
    fn ty(&self) -> TypeId;
    fn defined_at(&self) -> Location;
    fn uses(&self) -> Box<dyn Iterator<Item = Location> + '_>;
    fn is_public(&self) -> bool;
}
pub trait SymbolMut {
    fn access(&mut self) -> &mut SymbolAccessManager;
}
macro_rules! impl_symbol {
    ($symbol:ident) => {
        impl_symbol!($symbol, |_s: &$symbol| true);
    };

    ($symbol:ident, $public:expr) => {
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

            fn is_public(&self) -> bool {
                let f: fn(&$symbol) -> bool = $public;
                f(self)
            }
        }

        impl SymbolMut for $symbol {
            fn access(&mut self) -> &mut SymbolAccessManager {
                &mut self.access
            }
        }
    };
}
impl_symbol!(VariableSymbol, |s: &VariableSymbol| s.public);
impl_symbol!(FunctionSymbol, |s: &FunctionSymbol| s.public);
impl_symbol!(StructSymbol, |s: &StructSymbol| s.public);
impl_symbol!(EnumSymbol, |s: &EnumSymbol| s.public);
impl_symbol!(TypeAliasSymbol, |s: &TypeAliasSymbol| s.public);
impl_symbol!(MemberSymbol, |s: &MemberSymbol| s.public);
impl_symbol!(MethodSymbol, |s: &MethodSymbol| s.public);
impl_symbol!(TraitSymbol, |s: &TraitSymbol| s.public);

impl_symbol!(VariantSymbol); // uses default: always public
impl_symbol!(PrimitiveTypeSymbol); // uses default: always public
