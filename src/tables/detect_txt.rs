use crate::tables::model::TableCell;

pub fn detect_pipe_table(lines: &[&str]) -> Option<(Vec<TableCell>, usize, usize)> {
    if lines.len() < 2 {
        return None;
    }

    let header = split_pipe_row(lines[0])?;
    let separator = lines[1].trim();
    if !separator.contains("---") {
        return None;
    }

    let mut rows = vec![header.clone()];
    for line in &lines[2..] {
        if !line.contains('|') {
            break;
        }
        if let Some(cols) = split_pipe_row(line) {
            rows.push(cols);
        }
    }

    let columns = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if columns == 0 {
        return None;
    }

    let mut cells = Vec::new();
    for (r, row) in rows.iter().enumerate() {
        for c in 0..columns {
            let text = row.get(c).cloned().unwrap_or_default();
            cells.push(TableCell {
                row: r,
                column: c,
                rowspan: 1,
                colspan: 1,
                bbox: None,
                text,
                html: None,
                markdown: None,
                formula: None,
                is_header: r == 0,
                confidence: None,
            });
        }
    }

    Some((cells, rows.len(), columns))
}

pub fn detect_tsv_table(lines: &[&str]) -> Option<(Vec<TableCell>, usize, usize)> {
    let mut parsed = Vec::new();
    for line in lines {
        if !line.contains('\t') {
            break;
        }
        let cols = line.split('\t').map(|s| s.trim().to_string()).collect::<Vec<_>>();
        if cols.len() < 2 {
            break;
        }
        parsed.push(cols);
    }

    if parsed.len() < 2 {
        return None;
    }

    let columns = parsed.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut cells = Vec::new();
    for (r, row) in parsed.iter().enumerate() {
        for c in 0..columns {
            let text = row.get(c).cloned().unwrap_or_default();
            cells.push(TableCell {
                row: r,
                column: c,
                rowspan: 1,
                colspan: 1,
                bbox: None,
                text,
                html: None,
                markdown: None,
                formula: None,
                is_header: r == 0,
                confidence: None,
            });
        }
    }

    Some((cells, parsed.len(), columns))
}

fn split_pipe_row(line: &str) -> Option<Vec<String>> {
    if !line.contains('|') {
        return None;
    }
    let cols = line
        .trim()
        .trim_matches('|')
        .split('|')
        .map(|s| s.trim().to_string())
        .collect::<Vec<_>>();
    if cols.len() < 2 {
        None
    } else {
        Some(cols)
    }
}
