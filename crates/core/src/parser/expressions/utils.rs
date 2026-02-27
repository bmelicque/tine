pub fn is_type_identifier(text: &str) -> bool {
    match text {
        "bool" | "str" | "float" | "int" => true,
        _ => text
            .chars()
            .find(|c| *c != '_')
            .map_or(false, |c| c.is_uppercase()),
    }
}
