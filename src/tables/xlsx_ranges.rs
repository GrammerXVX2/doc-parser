#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRange {
    pub start_row: usize,
    pub end_row: usize,
    pub start_col: usize,
    pub end_col: usize,
}

pub fn detect_xlsx_table_ranges(rows: &[Vec<String>]) -> Vec<TableRange> {
    if rows.is_empty() {
        return vec![];
    }

    let width = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if width == 0 {
        return vec![];
    }

    let mut out = Vec::new();
    let mut start: Option<usize> = None;

    for (idx, row) in rows.iter().enumerate() {
        let non_empty = row.iter().any(|v| !v.trim().is_empty());
        match (start, non_empty) {
            (None, true) => start = Some(idx),
            (Some(s), false) => {
                out.push(TableRange {
                    start_row: s,
                    end_row: idx.saturating_sub(1),
                    start_col: 0,
                    end_col: width.saturating_sub(1),
                });
                start = None;
            }
            _ => {}
        }
    }

    if let Some(s) = start {
        out.push(TableRange {
            start_row: s,
            end_row: rows.len().saturating_sub(1),
            start_col: 0,
            end_col: width.saturating_sub(1),
        });
    }

    if out.is_empty() {
        return vec![TableRange {
            start_row: 0,
            end_row: rows.len().saturating_sub(1),
            start_col: 0,
            end_col: width.saturating_sub(1),
        }];
    }

    out
}
