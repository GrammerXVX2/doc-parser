use crate::tables::model::TableCell;

pub fn table_to_html(cells: &[TableCell], rows: usize, columns: usize) -> String {
    if rows == 0 || columns == 0 {
        return "<table></table>".to_string();
    }

    let mut grid: Vec<Vec<Option<&TableCell>>> = vec![vec![None; columns]; rows];
    for cell in cells {
        if cell.row < rows && cell.column < columns {
            grid[cell.row][cell.column] = Some(cell);
        }
    }

    let mut out = String::from("<table><tbody>");
    for row in grid {
        out.push_str("<tr>");
        for cell in row {
            match cell {
                Some(cell) => {
                    let tag = if cell.is_header { "th" } else { "td" };
                    let rowspan = if cell.rowspan > 1 {
                        format!(" rowspan=\"{}\"", cell.rowspan)
                    } else {
                        String::new()
                    };
                    let colspan = if cell.colspan > 1 {
                        format!(" colspan=\"{}\"", cell.colspan)
                    } else {
                        String::new()
                    };
                    out.push_str(&format!(
                        "<{tag}{rowspan}{colspan}>{}</{tag}>",
                        html_escape(&cell.text)
                    ));
                }
                None => out.push_str("<td></td>"),
            }
        }
        out.push_str("</tr>");
    }
    out.push_str("</tbody></table>");
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
