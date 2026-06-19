use crate::tables::model::TableCell;

pub fn table_to_csv(cells: &[TableCell], rows: usize, columns: usize) -> String {
    if rows == 0 || columns == 0 {
        return String::new();
    }

    let mut grid = vec![vec![String::new(); columns]; rows];
    for cell in cells {
        if cell.row < rows && cell.column < columns {
            grid[cell.row][cell.column] = escape_csv(&cell.text);
        }
    }

    grid.into_iter()
        .map(|row| row.join(","))
        .collect::<Vec<_>>()
        .join("\n")
}

fn escape_csv(input: &str) -> String {
    if input.contains(',') || input.contains('"') || input.contains('\n') {
        format!("\"{}\"", input.replace('"', "\"\""))
    } else {
        input.to_string()
    }
}
