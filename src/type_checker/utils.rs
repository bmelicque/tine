use std::collections::HashMap;

use crate::types;

use super::TypeChecker;

impl TypeChecker {
    pub fn check_dynamic_type(
        &self,
        ty: &types::Type,
        expected: &types::Type,
    ) -> Result<Box<types::Type>, String> {
        if matches!(ty, types::Type::Dynamic) {
            return Ok(Box::new(expected.clone()));
        }

        let e = Err(format!(
            "Type mismatch: expected {}, found {}",
            expected, ty
        ));

        if let types::Type::Struct(st) = ty {
            return if self.check_struct_type(st, expected) {
                Ok(Box::new(expected.clone()))
            } else {
                e
            };
        }
        if ty.is_assignable_to(expected) {
            Ok(Box::new(expected.clone()))
        } else {
            e
        }
    }

    fn check_struct_type(&self, ty: &types::StructType, expected: &types::Type) -> bool {
        let types::Type::Named(named) = expected else {
            return false;
        };

        let Some(found) = self.type_registry.lookup(&named.name) else {
            panic!("Named type should lead to a registered type!")
        };
        let types::Type::Struct(expected) = found else {
            return false;
        };

        let mut got = HashMap::new();
        ty.fields.iter().for_each(|field| {
            got.insert(&field.name, &field.def);
        });
        for field in expected.fields {
            let g = got.remove(&field.name);
            match (g, field.optional) {
                (Some(t), _) => {
                    if !t.is_assignable_to(&field.def) {
                        return false;
                    }
                }
                (None, false) => return false,
                (None, true) => {}
            }
        }

        got.len() == 0
    }
}
