use super::*;

pub(super) fn grid_arg<'gc>(c: Context<'gc>, spec: Table<'gc>) -> Result<Option<shared::PanelGrid>, Error<'gc>> {
    let rows = match spec.get(c, "rows") {
        Value::Nil => None,
        Value::Table(t) => Some(t),
        _ => return Err(fail(c, "bad_field: rows must be a table of rows".to_string())),
    };
    let columns = match spec.get(c, "columns") {
        Value::Nil => None,
        Value::Table(t) => Some(t),
        _ => return Err(fail(c, "bad_field: columns must be a table of strings".to_string())),
    };
    let Some(rows) = rows else {
        return match columns {
            None => Ok(None),
            Some(_) => Err(fail(c, "bad_field: columns needs rows".to_string())),
        };
    };
    if rows.length() as usize > shared::PANEL_MAX_ROWS {
        return Err(fail(c, format!("invalid_panel: more than {} rows", shared::PANEL_MAX_ROWS)));
    }
    let columns = columns.map(|t| cells_of(c, t, "columns")).transpose()?;
    let mut out = Vec::new();
    for k in 1..=rows.length() {
        match rows.get(c, k) {
            Value::Table(row) => out.push(cells_of(c, row, "rows")?),
            _ => return Err(fail(c, format!("bad_field: rows[{k}] must be a table of strings"))),
        }
    }
    Ok(Some(shared::PanelGrid { columns, rows: out }))
}

fn cells_of<'gc>(c: Context<'gc>, row: Table<'gc>, what: &str) -> Result<Vec<String>, Error<'gc>> {
    if row.length() as usize > shared::PANEL_MAX_COLUMNS {
        return Err(fail(c, format!("invalid_panel: {what} row has more than {} cells", shared::PANEL_MAX_COLUMNS)));
    }
    let mut cells = Vec::new();
    for k in 1..=row.length() {
        let cell = match row.get(c, k) {
            Value::String(s) => s.to_str().map(str::to_owned).map_err(|_| fail(c, format!("bad_field: {what} cell {k} is not utf8")))?,
            _ => return Err(fail(c, format!("bad_field: {what} cell {k} must be a string"))),
        };
        if cell.len() > shared::PANEL_MAX_CELL_BYTES {
            return Err(fail(c, format!("invalid_panel: {what} cell {k} is over {} bytes", shared::PANEL_MAX_CELL_BYTES)));
        }
        cells.push(cell);
    }
    Ok(cells)
}
