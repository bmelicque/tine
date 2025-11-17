use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use pest::Span;

use crate::types;

#[derive(Clone, Debug, PartialEq)]
pub struct VariableData {
    pub name: String,
    pub ty: types::TypeId,
    pub defined_at: Span<'static>,
    pub mutable: bool,
    pub reads: usize,
    pub writes: usize,
    pub mut_refs: usize,
    pub ro_refs: usize,
    pub dependencies: Vec<VariableRef>,
}

impl VariableData {
    pub fn new(
        name: String,
        ty: types::TypeId,
        mutable: bool,
        defined_at: Span<'static>,
        dependencies: Vec<VariableRef>,
    ) -> Self {
        Self {
            name,
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

    pub fn has_ref(&self) -> bool {
        self.mut_refs + self.ro_refs > 0
    }
}

#[derive(Debug, Clone)]
pub struct VariableHandle(Rc<RefCell<VariableData>>);

impl VariableHandle {
    pub fn new(symbol: VariableData) -> Self {
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

    pub fn borrow(&self) -> Ref<'_, VariableData> {
        self.0.borrow()
    }
    pub fn readonly(&self) -> VariableRef {
        VariableRef(self.0.clone())
    }

    pub fn has_ref(&self, variable: &VariableRef) -> bool {
        Rc::ptr_eq(&self.0, &variable.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableRef(Rc<RefCell<VariableData>>);

impl VariableRef {
    pub fn borrow(&self) -> Ref<'_, VariableData> {
        self.0.borrow()
    }
}
