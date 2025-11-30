use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use pest::Span;

use crate::{types::TypeId, utils::dummy_span, TypeStore};

#[derive(Clone, Debug, PartialEq)]
pub enum SymbolKind {
    Value {
        ty: TypeId,
        mutable: bool,
    },
    /// Contains the list of param names
    Function {
        params: Vec<String>,
        /// This TypeId should be guaranted to refer to a function type
        ty: TypeId,
    },
    Type(TypeId),
}

impl SymbolKind {
    pub fn constant(ty: TypeId) -> Self {
        Self::Value { ty, mutable: false }
    }

    pub fn get_type(&self) -> TypeId {
        match self {
            SymbolKind::Function { ty, .. } => *ty,
            SymbolKind::Type(ty) => *ty,
            SymbolKind::Value { ty, .. } => *ty,
        }
    }

    pub fn is_mutable(&self) -> bool {
        match self {
            SymbolKind::Value { mutable, .. } => *mutable,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SymbolData {
    pub name: String,
    pub kind: SymbolKind,
    pub defined_at: Span<'static>,
    pub docs: Option<String>,
    pub reads: usize,
    pub writes: usize,
    pub mut_refs: usize,
    pub ro_refs: usize,
    pub dependencies: Vec<SymbolRef>,
}

impl SymbolData {
    pub fn has_ref(&self) -> bool {
        self.mut_refs + self.ro_refs > 0
    }

    pub fn is_mutable(&self) -> bool {
        self.kind.is_mutable()
    }

    pub fn get_type(&self) -> TypeId {
        self.kind.get_type()
    }
}

impl Default for SymbolData {
    fn default() -> Self {
        Self {
            name: "".into(),
            kind: SymbolKind::Value {
                ty: TypeStore::UNKNOWN,
                mutable: false,
            },
            defined_at: dummy_span(),
            docs: None,
            reads: 0,
            writes: 0,
            mut_refs: 0,
            ro_refs: 0,
            dependencies: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct SymbolHandle(Rc<RefCell<SymbolData>>);

impl SymbolHandle {
    pub fn new(symbol: SymbolData) -> Self {
        Self(Rc::new(RefCell::new(symbol)))
    }

    pub fn is_mutable(&self) -> bool {
        match self.0.borrow().kind {
            SymbolKind::Value { mutable, .. } => mutable,
            _ => false,
        }
    }

    pub fn add_read(&self) {
        self.0.borrow_mut().reads += 1;
    }
    pub fn remove_read(&self) {
        self.0.borrow_mut().reads -= 1;
    }
    pub fn add_write(&self) {
        self.0.borrow_mut().writes += 1;
    }
    pub fn add_readonly_ref(&self) {
        self.0.borrow_mut().ro_refs += 1;
    }
    pub fn add_mutable_ref(&self) {
        self.0.borrow_mut().mut_refs += 1;
    }

    pub fn borrow(&self) -> Ref<'_, SymbolData> {
        self.0.borrow()
    }
    pub fn readonly(&self) -> SymbolRef {
        SymbolRef(self.0.clone())
    }

    pub fn has_ref(&self, variable: &SymbolRef) -> bool {
        Rc::ptr_eq(&self.0, &variable.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SymbolRef(Rc<RefCell<SymbolData>>);

impl SymbolRef {
    pub fn borrow(&self) -> Ref<'_, SymbolData> {
        self.0.borrow()
    }
}
