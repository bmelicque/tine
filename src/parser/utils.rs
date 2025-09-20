pub fn is_pascal_case(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().map(|c| c.is_uppercase()).unwrap_or(false)
}

// TODO: use this in parse_binary_expression_ltr
pub fn merge_span(a: pest::Span<'static>, b: pest::Span<'static>) -> pest::Span<'static> {
    pest::Span::new(a.get_input(), a.start(), b.end()).unwrap()
}
