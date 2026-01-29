use crate::{analyzer::Source, parser::parser::ParseError};

pub fn pretty_print_error(src: &Source, error: &ParseError) {
    let loc = &error.loc;
    let start_pos = loc.span().start();
    let end_pos = loc.span().end();

    let (start_line, start_col) = src.line_col(start_pos);
    let line_text = src.read_line(start_line);
    let end_col = match src.line_col(end_pos) {
        (line, col) if line == start_line => col,
        (_, _) => line_text.len(),
    };

    println!(
        "\nerror: {}\n --> line {}, column {}\n",
        error.message, start_line, start_col
    );
    println!("{} | {}", start_line, line_text);

    let gutter = " ".repeat(start_line.to_string().len());
    let underline = if end_col > start_col {
        "~".repeat(end_col - start_col)
    } else {
        "^".to_string()
    };

    println!("{} | {}{}", gutter, " ".repeat(start_col - 1), underline);
}
