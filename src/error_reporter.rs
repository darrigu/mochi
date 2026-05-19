pub fn report_errors(source: &str, errors: &[crate::parser::Diagnostic]) {
    let lines: Vec<&str> = source.lines().collect();

    for err in errors {
        println!("\n\x1b[31;1merror\x1b[0m: {}", err.message);

        let line_num_str = err.line.to_string();
        let gutter_padding = " ".repeat(line_num_str.len());

        println!(
            "\x1b[34;1m  {} ╭─▶\x1b[0m line {}:{}",
            gutter_padding, err.line, err.col
        );

        if let Some(line_content) = lines.get(err.line - 1) {
            println!("\x1b[34;1m  {} │\x1b[0m", gutter_padding);
            println!("\x1b[34;1m  {} │\x1b[0m {}", err.line, line_content);
            print!("\x1b[34;1m  {} │\x1b[0m ", gutter_padding);

            for (i, ch) in line_content.chars().enumerate() {
                if i + 1 == err.col {
                    break;
                }
                if ch == '\t' {
                    print!("    ");
                } else {
                    print!(" ");
                }
            }
            println!("\x1b[31;1m╰─▶\x1b[0m");
        }

        if let Some(hint) = &err.hint {
            println!("\x1b[34;1m  {} │\x1b[0m", gutter_padding);
            println!(
                "\x1b[34;1m  {} ╰─=\x1b[0m \x1b[36;1mhint\x1b[0m: {}",
                gutter_padding, hint
            );
        } else {
            println!("\x1b[34;1m  {} ╰─\x1b[0m", gutter_padding);
        }
    }
    println!();
}

pub fn report_system_error(stage: &str, msg: &str) {
    println!("\n\x1b[31;1m{} error\x1b[0m: {}", stage, msg);
    println!("\x1b[34;1m    ╰─=\x1b[0m Execution aborted.\n");
}
