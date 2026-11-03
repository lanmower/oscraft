use super::*;

pub(super) fn bind_editor<'gc>(ctx: Context<'gc>, game: Table<'gc>, sh: &Sh) {
    native!(ctx, game, "edit_terrain", [sh], |c, s| {
        let (x, y) = cell_args(c, &s)?;
        let blocked = s.get(2).to_bool();
        let mut sh = sh.borrow_mut();
        check_cell(c, &sh.sim, x, y)?;
        sh.cmds.push(Cmd::EditCell { x, y, blocked });
        s.clear();
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_tile", [sh], |c, s| {
        let (x, y) = cell_args(c, &s)?;
        let tile = u8::try_from(a(c, &s, 2)?).map_err(|_| fail(c, "invalid_tile: a tile id is 0..=255".into()))?;
        let mut sh = sh.borrow_mut();
        check_cell(c, &sh.sim, x, y)?;
        sh.cmds.push(Cmd::SetTile { x, y, tile });
        s.clear();
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "tile", [sh], |c, s| {
        let (x, y) = cell_args(c, &s)?;
        let tile = sh.borrow().sim.grid().tile(x, y);
        s.replace(c, tile.map_or(Value::Nil, |t| Value::Integer(i64::from(t))));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "list_abilities", [sh], |c, s| {
        let prefix = match s.get(0) {
            Value::Nil => String::new(),
            v => text(c, v, 0)?,
        };
        let list = Table::new(&c);
        let sh = sh.borrow();
        let cat = sh.sim.catalog();
        let mut n = 0i64;
        for (index, ab) in cat.abilities.iter().enumerate() {
            let name = cat.name(ab.name);
            if !name.starts_with(&prefix) {
                continue;
            }
            let row = Table::new(&c);
            row.put(c, "index", index as i64);
            row.put(c, "name", name.to_owned());
            row.put(c, "cooldown", i64::from(ab.cooldown));
            row.put(c, "energy", i64::from(ab.energy));
            row.put(c, "flags", i64::from(ab.flags));
            n += 1;
            list.put(c, n, row);
        }
        s.replace(c, list);
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "list_events", [sh], |c, s| {
        let list = Table::new(&c);
        for (k, (name, args)) in EVENTS.iter().enumerate() {
            let row = Table::new(&c);
            row.put(c, "name", (*name).to_owned());
            row.put(c, "args", (*args).to_owned());
            list.put(c, k as i64 + 1, row);
        }
        s.replace(c, list);
        Ok(CallbackReturn::Return)
    });
}

fn cell_args<'gc>(c: Context<'gc>, s: &Stack<'gc, '_>) -> Result<(i32, i32), Error<'gc>> {
    let x = i32::try_from(a(c, s, 0)?).map_err(|_| fail(c, "invalid_cell: x out of range".into()))?;
    let y = i32::try_from(a(c, s, 1)?).map_err(|_| fail(c, "invalid_cell: y out of range".into()))?;
    Ok((x, y))
}

fn check_cell<'gc>(c: Context<'gc>, sim: &Sim, x: i32, y: i32) -> Result<(), Error<'gc>> {
    if sim.grid().tile(x, y).is_none() {
        return Err(fail(c, format!("invalid_cell: ({x}, {y}) is outside the grid")));
    }
    Ok(())
}
