use std::collections::HashMap;

use crate::ast;

use super::CodeGenerator;

pub struct Scope {
    layers: Vec<HashMap<String, Vec<String>>>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            layers: vec![HashMap::new()],
        }
    }

    pub fn register(&mut self, name: String, sorted: Vec<String>) {
        self.layers.last_mut().unwrap().insert(name, sorted);
    }

    pub fn find(&self, name: &String) -> Option<&Vec<String>> {
        for layer in self.layers.iter().rev() {
            let got = layer.get(name);
            if got.is_some() {
                return got;
            }
        }
        return None;
    }

    pub fn enter(&mut self) {
        self.layers.push(HashMap::new());
    }
    pub fn exit(&mut self) {
        self.layers.pop();
        assert!(self.layers.len() > 0, "should not remove last scope layer!")
    }
}

impl CodeGenerator {
    pub fn register_struct(&mut self, name: &String, fields: &Vec<ast::StructDefinitionField>) {
        let mandatory_fields = fields.iter().filter(|field| !field.is_optional());
        let optional_fields = fields.iter().filter(|field| field.is_optional());

        let sorted_names = mandatory_fields
            .chain(optional_fields)
            .map(|field| field.as_name())
            .collect::<Vec<_>>();

        self.add_to_scope(name.clone(), sorted_names);
    }
}
