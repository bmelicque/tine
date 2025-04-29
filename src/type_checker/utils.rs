use std::collections::HashMap;

use crate::types::Type;

use super::TypeChecker;

impl TypeChecker {
    pub fn check_dynamic_type(&self, ty: &Type, expected: &Type) -> Result<Box<Type>, String> {
        if matches!(ty, Type::Dynamic) {
            return Ok(Box::new(expected.clone()));
        }

        let e = Err(format!(
            "Type mismatch: expected {}, found {}",
            expected, ty
        ));

        if let Type::Struct { .. } = ty {
            return if self.check_struct_type(ty, expected) {
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

    fn check_struct_type(&self, ty: &Type, expected: &Type) -> bool {
        let Type::Struct { ref fields } = ty else {
            panic!();
        };

        let Type::Named { name, .. } = expected else {
            return false;
        };

        let Some(found) = self.type_registry.lookup(name) else {
            panic!("Named type should lead to a registered type!")
        };
        let Type::Struct {
            fields: ref expected_fields,
        } = found
        else {
            return false;
        };

        let mut got = HashMap::new();
        for field in fields {
            got.insert(&field.name, &field.def);
        }
        for field in expected_fields {
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
