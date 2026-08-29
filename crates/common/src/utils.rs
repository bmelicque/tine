use crate::{diagnostics::Diagnostic, sources::Source};

pub fn pretty_print_error(src: &Source, diag: &Diagnostic) {
    let loc = &diag.loc;
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
        diag.kind,
        start_line + 1,
        start_col + 1
    );
    println!("{} | {}", start_line + 1, line_text.trim_end());

    let gutter = " ".repeat((start_line + 1).to_string().len());
    let underline = if end_col > start_col {
        "~".repeat(end_col - start_col)
    } else {
        "^".to_string()
    };

    println!("{} | {}{}", gutter, " ".repeat(start_col), underline);
}
