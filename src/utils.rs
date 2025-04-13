use crate::parser::parser::ParseError;

pub fn pretty_print_error(source: &str, error: &ParseError) {
    let span = &error.span;
    let start_pos = span.start_pos();
    let end_pos = span.end_pos();

    let line_str = start_pos.line_col().0;
    let col_start = start_pos.line_col().1;
    let col_end = end_pos.line_col().1;

    let line_text = source
        .lines()
        .nth(line_str - 1) // lines are 1-based
        .unwrap_or("");

    println!(
        "\nerror: {}\n --> line {}, column {}\n",
        error.message, line_str, col_start
    );
    println!("{} | {}", line_str, line_text);

    let gutter = " ".repeat(line_str.to_string().len());
    let underline = if col_end > col_start {
        "~".repeat(col_end - col_start)
    } else {
        "^".to_string()
    };

    println!("{} | {}{}", gutter, " ".repeat(col_start - 1), underline);
}
