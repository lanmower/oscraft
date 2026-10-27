use super::*;

pub(super) fn bind_scaling<'gc>(ctx: Context<'gc>, game: Table<'gc>, sh: &Sh) {
    native!(ctx, game, "set_xp_rate", [sh], |c, s| {
        let player = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let percent = percent_arg(c, a(c, &s, 1)?)?;
        sh.borrow_mut().cmds.push(Cmd::SetXpRate { player, percent });
        s.clear();
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "xp_rate", [sh], |c, s| {
        let player = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let rate = sh.borrow().sim.xp_rate(usize::from(player));
        s.replace(c, i64::from(rate));
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "set_unit_scale", [sh], |c, s| {
        let unit = a(c, &s, 0)?;
        let percent = percent_arg(c, a(c, &s, 1)?)?;
        let mut sh = sh.borrow_mut();
        let unit = known_unit(&sh.sim, unit).map_err(|m| fail(c, m))?;
        sh.cmds.push(Cmd::SetUnitScale { unit, percent });
        s.clear();
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "unit_scale", [sh], |c, s| {
        let unit = known_unit(&sh.borrow().sim, a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let scale = sh.borrow().sim.unit_scale(unit);
        s.replace(c, i64::from(scale));
        Ok(CallbackReturn::Return)
    });
}

fn percent_arg<'gc>(c: Context<'gc>, value: i64) -> Result<u16, Error<'gc>> {
    u16::try_from(value)
        .ok()
        .filter(|&p| p <= 1000)
        .ok_or_else(|| fail(c, format!("invalid_percent: {value} is outside 0..=1000")))
}
