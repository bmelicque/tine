pub fn merge_span<'i>(a: pest::Span<'i>, b: pest::Span<'i>) -> pest::Span<'i> {
    let start = if a.start() < b.start() {
        a.start()
    } else {
        b.start()
    };
    let end = if a.end() < b.end() { a.end() } else { b.end() };
    pest::Span::new(a.as_str(), start, end).unwrap()
}
