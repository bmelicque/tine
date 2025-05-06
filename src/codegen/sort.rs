use std::collections::HashMap;

use crate::ast;

use super::CodeGenerator;

pub struct Scope {
    symbols: HashMap<String, Vec<String>>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: String, sorted: Vec<String>) {
        self.symbols.insert(name, sorted);
    }

    pub fn get(&self, name: &String) -> Option<&Vec<String>> {
        self.symbols.get(name)
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
