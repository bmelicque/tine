use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use pest::Span;

use crate::types::{self, TypeId};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SymbolKind {
    Value,
    Type,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SymbolData {
    pub name: String,
    pub kind: SymbolKind,
    pub ty: types::TypeId,
    pub defined_at: Span<'static>,
    pub mutable: bool,
    pub reads: usize,
    pub writes: usize,
    pub mut_refs: usize,
    pub ro_refs: usize,
    pub dependencies: Vec<SymbolRef>,
}

impl SymbolData {
    pub fn new(
        name: String,
        kind: SymbolKind,
        ty: types::TypeId,
        mutable: bool,
        defined_at: Span<'static>,
        dependencies: Vec<SymbolRef>,
    ) -> Self {
        Self {
            name,
            kind,
            ty,
            defined_at,
            mutable,
            reads: 0,
            writes: 0,
            mut_refs: 0,
            ro_refs: 0,
            dependencies,
        }
    }

    pub fn pure(name: String, ty: types::TypeId, defined_at: Span<'static>) -> Self {
        Self {
            name,
            kind: SymbolKind::Value,
            ty,
            defined_at,
            mutable: false,
            reads: 0,
            writes: 0,
            mut_refs: 0,
            ro_refs: 0,
            dependencies: vec![],
        }
    }

    pub fn new_type(name: String, type_id: TypeId, defined_at: Span<'static>) -> Self {
        Self {
            name,
            kind: SymbolKind::Type,
            ty: type_id,
            defined_at,
            mutable: false,
            reads: 0,
            writes: 0,
            mut_refs: 0,
            ro_refs: 0,
            dependencies: vec![],
        }
    }

    pub fn has_ref(&self) -> bool {
        self.mut_refs + self.ro_refs > 0
    }
}

#[derive(Debug, Clone)]
pub struct SymbolHandle(Rc<RefCell<SymbolData>>);

impl SymbolHandle {
    pub fn new(symbol: SymbolData) -> Self {
        Self(Rc::new(RefCell::new(symbol)))
    }

    pub fn is_mutable(&self) -> bool {
        self.0.borrow().mutable
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
