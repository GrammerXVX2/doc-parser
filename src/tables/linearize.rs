use crate::model::Element;
use crate::tables::model::{TableCell, TableLinearizationOptions, TableLinearizedChunk};

pub fn linearize_table(
    table: &Element,
    options: TableLinearizationOptions,
) -> Vec<TableLinearizedChunk> {
    let rows = table
        .extra
        .get("rows")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(0);
    let columns = table
        .extra
        .get("columns")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(0);

    let cells = table
        .extra
        .get("cells")
        .and_then(|v| serde_json::from_value::<Vec<TableCell>>(v.clone()).ok())
        .unwrap_or_default();

    linearize_cells(&cells, rows, columns, options)
}

pub fn linearize_cells(
    cells: &[TableCell],
    rows: usize,
    columns: usize,
    options: TableLinearizationOptions,
) -> Vec<TableLinearizedChunk> {
    if rows == 0 || columns == 0 {
        return vec![];
    }

    let max_rows = options.max_rows_per_chunk.max(1);
    let mut grid = vec![vec![String::new(); columns]; rows];
    let mut headers = vec![String::new(); columns];
    for cell in cells {
        if cell.row < rows && cell.column < columns {
            grid[cell.row][cell.column] = cell.text.clone();
            if cell.is_header {
                headers[cell.column] = cell.text.clone();
            }
        }
    }
    if headers.iter().all(|h| h.trim().is_empty()) && rows > 0 {
        headers.clone_from_slice(&grid[0]);
    }

    let mut out = Vec::new();
    let start_row = if rows > 1 { 1 } else { 0 };
    let mut chunk_start = start_row;
    while chunk_start < rows {
        let chunk_end = (chunk_start + max_rows).min(rows);
        let mut lines = Vec::new();
        for r in chunk_start..chunk_end {
            let mut parts = Vec::new();
            for c in 0..columns {
                let key = headers.get(c).cloned().unwrap_or_else(|| format!("Колонка {}", c + 1));
                let value = grid[r][c].clone();
                parts.push(format!("{} = {}", key, value));
            }
            lines.push(format!("Строка {}: {}.", r + 1, parts.join("; ")));
        }

        out.push(TableLinearizedChunk {
            title: "Таблица".to_string(),
            text: lines.join("\n"),
            markdown: String::new(),
            row_start: chunk_start,
            row_end: chunk_end.saturating_sub(1),
        });

        chunk_start = chunk_end;
    }

    out
}
