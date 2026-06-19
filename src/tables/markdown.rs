use crate::tables::model::TableCell;

pub fn table_to_markdown(cells: &[TableCell], rows: usize, columns: usize) -> String {
    if rows == 0 || columns == 0 {
        return String::new();
    }

    let mut grid = vec![vec![String::new(); columns]; rows];
    let mut header_flags = vec![false; columns];
    for cell in cells {
        if cell.row < rows && cell.column < columns {
            grid[cell.row][cell.column] = cell.text.clone();
            if cell.is_header {
                header_flags[cell.column] = true;
            }
        }
    }

    let header_row = if header_flags.iter().any(|v| *v) { 0 } else { 0 };
    let header = format!("| {} |", grid[header_row].join(" | "));
    let separator = format!("|{}|", vec!["---"; columns].join("|"));

    let mut lines = vec![header, separator];
    for (idx, row) in grid.into_iter().enumerate() {
        if idx == header_row {
            continue;
        }
        lines.push(format!("| {} |", row.join(" | ")));
    }

    lines.join("\n")
}
