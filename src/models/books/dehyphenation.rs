pub fn apply_dehyphenation(input: &str) -> (String, bool) {
    let mut out = String::new();
    let mut changed = false;
    let mut in_code_block = false;

    let mut lines = input.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            out.push_str(line);
            out.push('\n');
            continue;
        }

        let skip_line = in_code_block
            || trimmed.contains("http://")
            || trimmed.contains("https://")
            || trimmed.contains('|')
            || trimmed.contains('$');

        if !skip_line && trimmed.ends_with('-') {
            let current = line.trim_end_matches('-');
            if let Some(next_line) = lines.peek() {
                let next_trimmed = next_line.trim_start();
                let next_skip = next_trimmed.contains("http://")
                    || next_trimmed.contains("https://")
                    || next_trimmed.contains('|')
                    || next_trimmed.contains('$');
                if !next_skip {
                    out.push_str(current);
                    out.push_str(next_trimmed);
                    out.push('\n');
                    let _ = lines.next();
                    changed = true;
                    continue;
                }
            }
        }

        out.push_str(line);
        out.push('\n');
    }

    (out, changed)
}
