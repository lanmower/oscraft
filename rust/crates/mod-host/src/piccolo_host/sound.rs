use super::*;

pub(super) fn bind_sound<'gc>(ctx: Context<'gc>, game: Table<'gc>, sh: &Sh) {
    native!(ctx, game, "sound", [sh], |c, s| {
        let name = text(c, s.get(0), 0)?;
        let player = if matches!(s.get(1), Value::Nil) { -1 } else { a(c, &s, 1)? };
        let body = shared::ui_sound(&name, player).map_err(|m| fail(c, m))?;
        sh.borrow_mut().cmds.push(Cmd::Ui { body });
        s.clear();
        Ok(CallbackReturn::Return)
    });
}
