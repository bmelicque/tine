pub fn is_camel_case(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().map(|c| c.is_lowercase()).unwrap_or(false)
}

pub fn is_pascal_case(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().map(|c| c.is_uppercase()).unwrap_or(false)
}
