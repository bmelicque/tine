use std::sync::{Arc, Mutex, MutexGuard};

use crate::{types::TypeId, Location, TypeStore};

#[derive(Clone, Debug)]
pub enum TypeSymbolBody {
    Struct(Vec<(String, SymbolRef)>),
    Tuple(Vec<SymbolRef>),
}

#[derive(Clone, Debug)]
pub enum SymbolKind {
    Value {
        mutable: bool,
    },
    /// A function name.
    /// Its type_id should refer to a FunctionType.
    Function {
        // This is expected to have the same length as the function type's params.
        param_names: Vec<String>,
    },
    PrimitiveType {
        methods: Vec<SymbolRef>,
    },
    TypeAlias,
    Struct {
        body: TypeSymbolBody,
        methods: Vec<SymbolRef>,
    },
    Enum {
        /// All the variants of the enum.
        /// This should only contain `Constructor` symbols
        variants: Vec<SymbolRef>,
        methods: Vec<SymbolRef>,
    },
    /// An enum constructor. In this case, the symbol's `def` should refer to either a `StructType`, a `TupleType` or a `TypeTemplate` wrapping either
    Constructor {
        /// The enum symbol owning this.
        owner: SymbolRef,
        body: Option<TypeSymbolBody>,
    },
    Member {
        /// The type definition of the struct owning this member.
        owner: SymbolRef,
    },
    Method {
        /// The type definition of the type owning this.
        owner: SymbolRef,
        /// All the type arguments on the receiver.
        ///
        /// eg in `Type<Arg1, Arg2>.staticMethod()`
        /// (same for instance methods)
        owner_args: Vec<TypeId>,
        /// If `has_receiver`, then it's an instance method. Else, it's a static method.
        has_receiver: bool,
        // This is expected to have the same length as the function type's params.
        param_names: Vec<String>,
    },
}

impl SymbolKind {
    pub fn constant() -> Self {
        Self::Value { mutable: false }
    }

    pub fn is_mutable(&self) -> bool {
        match self {
            SymbolKind::Value { mutable, .. } => *mutable,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
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
}

#[derive(Clone, Debug)]
pub struct SymbolData {
    pub name: String,
    pub ty: TypeId,
    pub kind: SymbolKind,
    pub defined_at: Location,
    pub docs: Option<String>,
    pub access: SymbolAccessManager,
    pub dependencies: Vec<SymbolRef>,
}

impl SymbolData {
    pub fn has_ref(&self) -> bool {
        self.access.references.len() > 0
    }

    pub fn is_mutable(&self) -> bool {
        self.kind.is_mutable()
    }

    pub fn get_type(&self) -> TypeId {
        self.ty
    }

    pub fn is_type_symbol(&self) -> bool {
        matches!(
            self.kind,
            SymbolKind::Struct { .. } | SymbolKind::Enum { .. }
        )
    }
}

impl Default for SymbolData {
    fn default() -> Self {
        Self {
            name: "".into(),
            ty: TypeStore::UNKNOWN,
            kind: SymbolKind::constant(),
            defined_at: Location::dummy(),
            docs: None,
            access: SymbolAccessManager::new(),
            dependencies: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct SymbolHandle(Arc<Mutex<SymbolData>>);

impl SymbolHandle {
    pub fn new(symbol: SymbolData) -> Self {
        Self(Arc::new(Mutex::new(symbol)))
    }

    pub fn is_mutable(&self) -> bool {
        match self.0.lock().unwrap().kind {
            SymbolKind::Value { mutable, .. } => mutable,
            _ => false,
        }
    }

    pub fn read(&self, at: Location) {
        self.0.lock().unwrap().access.read(at);
    }
    pub fn reference(&self, at: Location) {
        self.0.lock().unwrap().access.references.push(at);
    }
    pub fn write(&self, at: Location) {
        self.0.lock().unwrap().access.writes.push(at);
    }
    pub fn read_to_write(&self, at: Location) {
        self.0.lock().unwrap().access.reads.retain(|loc| *loc != at);
        self.0.lock().unwrap().access.writes.push(at);
    }
    pub fn read_to_mutable_ref(&self, at: Location) {
        self.read_to_write(at);
        self.reference(at);
    }

    pub fn borrow(&self) -> MutexGuard<'_, SymbolData> {
        self.0.lock().unwrap()
    }
    pub fn readonly(&self) -> SymbolRef {
        SymbolRef(self.0.clone())
    }

    pub fn has_ref(&self, variable: &SymbolRef) -> bool {
        Arc::ptr_eq(&self.0, &variable.0)
    }
}

#[derive(Debug, Clone)]
pub struct SymbolRef(Arc<Mutex<SymbolData>>);

impl SymbolRef {
    pub fn borrow(&self) -> MutexGuard<'_, SymbolData> {
        self.0.lock().unwrap()
    }

    pub fn as_name(&self) -> String {
        self.borrow().name.clone()
    }

    pub fn as_type(&self) -> TypeId {
        self.borrow().ty
    }

    pub fn as_type_body(&self) -> Option<TypeSymbolBody> {
        match &self.borrow().kind {
            SymbolKind::Struct { body, .. } => Some(body.clone()),
            SymbolKind::Constructor { body, .. } => body.clone(),
            _ => None,
        }
    }

    pub fn as_variants(&self) -> Option<Vec<SymbolRef>> {
        match &self.borrow().kind {
            SymbolKind::Enum { variants, .. } => Some(variants.clone()),
            _ => None,
        }
    }

    pub fn is(&self, test: &SymbolRef) -> bool {
        Arc::ptr_eq(&self.0, &test.0)
    }

    pub fn uses(&self) -> Vec<Location> {
        let symbol = self.0.lock().unwrap();
        vec![symbol.defined_at]
            .into_iter()
            .chain(symbol.access.uses())
            .collect::<Vec<_>>()
    }

    pub fn is_referenced(&self) -> bool {
        self.borrow().access.references.len() > 0
    }

    pub fn is_scalar(&self) -> bool {
        match self.as_type() {
            TypeStore::BOOLEAN | TypeStore::FLOAT | TypeStore::INTEGER | TypeStore::STRING => true,
            _ => false,
        }
    }
}
impl PartialEq for SymbolRef {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
