use crate::{
    ast::{self, StructDefinitionField},
    parser::parser::ParseError,
    types::{StructField, SumVariant, TraitMethod, Type},
};

use super::{scopes::TypeMetadata, TypeChecker};

impl TypeChecker {
    pub fn visit_type_declaration(&mut self, node: &ast::TypeAlias) -> Type {
        if let Some(ref type_params) = node.params {
            for type_param in type_params {
                self.type_registry.define_generic(&type_param);
            }
        }
        let ty = self.visit_type_definition(&node.definition);
        self.type_registry.clear_generics();

        let name = &node.name;
        match self.type_registry.lookup(name) {
            Some(_) => self.errors.push(ParseError {
                message: format!("Type {} already defined", name),
                span: node.span,
            }),
            None => {
                let metadata = node.params.clone().map(|params| TypeMetadata {
                    type_params: params,
                });
                self.type_registry.define(&name, ty, metadata);
            }
        }

        Type::Void
    }

    fn visit_type_definition(&mut self, node: &ast::TypeDefinition) -> Type {
        match node {
            ast::TypeDefinition::Enum(e) => self.visit_enum_definition(e),
            ast::TypeDefinition::Struct(s) => self.visit_struct_definition(s),
            ast::TypeDefinition::Trait(t) => self.visit_trait_definition(t),
            ast::TypeDefinition::Type(t) => self.visit_type(t),
        }
    }

    fn visit_enum_definition(&mut self, node: &ast::EnumDefinition) -> Type {
        let variants = node
            .variants
            .iter()
            .map(|variant| self.visit_variant_definition(variant))
            .collect();

        Type::Sum { variants }
    }

    fn visit_variant_definition(&mut self, node: &ast::VariantDefinition) -> SumVariant {
        let def = match node {
            ast::VariantDefinition::Struct(s) => self.visit_struct_definition(&s.def),
            ast::VariantDefinition::Tuple(t) => {
                let elements = t.elements.iter().map(|el| self.visit_type(el)).collect();
                Type::Tuple(elements)
            }
            ast::VariantDefinition::Unit(_) => Type::Unit,
        };
        SumVariant {
            name: node.as_name(),
            def,
        }
    }

    fn visit_struct_definition(&mut self, node: &ast::StructDefinition) -> Type {
        let fields = node
            .fields
            .iter()
            .map(|field| self.visit_struct_definition_field(field))
            .collect();
        Type::Struct { fields }
    }

    fn visit_struct_definition_field(&mut self, field: &ast::StructDefinitionField) -> StructField {
        let name = field.as_name();
        let def = match field {
            ast::StructDefinitionField::Mandatory(ref field) => self.visit_type(&field.definition),
            ast::StructDefinitionField::Optional(field) => self.visit_expression(&field.default),
        };
        StructField {
            name,
            def,
            optional: field.is_optional(),
        }
    }

    fn visit_trait_definition(&mut self, node: &ast::TraitDefinition) -> Type {
        self.type_registry.current_self = Some(node.name.clone());

        let method_types = node
            .body
            .fields
            .iter()
            .filter_map(|field| self.visit_trait_method_definition(field))
            .collect();

        self.type_registry.current_self = None;

        Type::Trait {
            methods: method_types,
        }
    }

    fn visit_trait_method_definition(
        &mut self,
        node: &StructDefinitionField,
    ) -> Option<TraitMethod> {
        let as_field = self.visit_struct_definition_field(node);
        if !matches!(as_field.def, Type::Function { .. }) {
            self.errors.push(ParseError {
                message: format!(
                    "Only methods are allowed in trait definitions, found {}",
                    as_field.def
                ),
                span: node.as_span(),
            });
            return None;
        }
        Some(TraitMethod {
            name: as_field.name,
            def: as_field.def,
        })
    }
}
