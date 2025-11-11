pub fn is_pascal_case(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().map(|c| c.is_uppercase()).unwrap_or(false)
}

// TODO: use this in parse_binary_expression_ltr
pub fn merge_span(a: pest::Span<'static>, b: pest::Span<'static>) -> pest::Span<'static> {
    pest::Span::new(a.get_input(), a.start(), b.end()).unwrap()
}

/// Make a 1 char long span starting just after the end of given span
pub fn increment_span(span: pest::Span<'static>) -> pest::Span<'static> {
    let str = span.get_input();
    let end = span.end();
    if end >= str.len() {
        pest::Span::new(str, end - 1, end).unwrap()
    } else {
        pest::Span::new(str, end, end + 1).unwrap()
    }
}
