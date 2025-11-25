pub fn normalize_doc_comment(input: &str) -> String {
    let mut paragraphs = Vec::new();
    let mut current = Vec::new();

    for line in input.lines() {
        let line = line
            .trim_start()
            .strip_prefix("//")
            .unwrap_or(line)
            .trim_start();

        if !line.is_empty() {
            current.push(line.to_string());
        } else if !current.is_empty() {
            paragraphs.push(current.join(" "));
            current.clear();
        }
    }

    if !current.is_empty() {
        paragraphs.push(current.join(" "));
    }

    paragraphs.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::normalize_doc_comment;

    #[test]
    fn test_normalize_single_line_comment() {
        let input = "// Hello world";
        let out = normalize_doc_comment(input);
        assert_eq!(out, "Hello world");
    }

    #[test]
    fn test_normalize_multi_line_comment_into_paragraph() {
        let input = "\
        // First line
        // Second line

        // New paragraph
        // continues";
        let out = normalize_doc_comment(input);
        let expected = "First line Second line\n\nNew paragraph continues";
        assert_eq!(out, expected);
    }

    #[test]
    fn test_trims_leading_whitespace_and_slashes() {
        let input = "   //    indented\n\t//\tmore";
        let out = normalize_doc_comment(input);
        assert_eq!(out, "indented more");
    }
}
