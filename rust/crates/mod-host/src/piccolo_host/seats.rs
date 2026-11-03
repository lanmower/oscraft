use std::collections::BTreeMap;

use super::*;

const NAME_MAX_BYTES: usize = 32;
const KEY_MAX_BYTES: usize = 32;
const SETTINGS_MAX: usize = 64;

#[derive(Default)]
pub(super) struct SeatState {
    names: Vec<Option<String>>,
    colours: Vec<Option<i64>>,
    settings: Vec<BTreeMap<String, i64>>,
}

impl SeatState {
    fn slot<T: Default>(list: &mut Vec<T>, player: u8) -> &mut T {
        let at = usize::from(player);
        if list.len() <= at {
            list.resize_with(MAX_PLAYERS, T::default);
        }
        &mut list[at]
    }

    fn name(&self, player: u8) -> String {
        match self.names.get(usize::from(player)) {
            Some(Some(name)) => name.clone(),
            _ => format!("Player {}", u32::from(player) + 1),
        }
    }

    fn colour(&self, player: u8) -> Option<i64> {
        self.colours.get(usize::from(player)).copied().flatten()
    }

    fn setting(&self, player: u8, key: &str) -> Option<i64> {
        self.settings.get(usize::from(player)).and_then(|m| m.get(key)).copied()
    }
}

fn valid_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > NAME_MAX_BYTES || name.chars().any(char::is_control) {
        return Err(format!("invalid_name: a seat name is 1..{NAME_MAX_BYTES} bytes without control characters"));
    }
    Ok(())
}

fn valid_key(key: &str) -> Result<(), String> {
    if key.is_empty() || key.len() > KEY_MAX_BYTES || !key.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') {
        return Err(format!("invalid_setting: key {key:?} must be lowercase letters, digits and underscores, at most {KEY_MAX_BYTES} bytes"));
    }
    Ok(())
}

fn seat_event(player: u8, name: &str, colour: Option<i64>) -> String {
    let colour = colour.map_or_else(|| "null".to_owned(), |c| c.to_string());
    format!(r#""kind":"seat","player":-1,"seat":{player},"name":{},"colour":{colour}"#, shared::ui_json_string(name))
}

pub(super) fn bind_seats<'gc>(ctx: Context<'gc>, game: Table<'gc>, sh: &Sh) {
    native!(ctx, game, "player_name", [sh], |c, s| {
        let p = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        if matches!(s.get(1), Value::Nil) {
            let name = sh.borrow().seats.name(p);
            s.replace(c, name);
            return Ok(CallbackReturn::Return);
        }
        let name = text(c, s.get(1), 1)?;
        valid_name(&name).map_err(|m| fail(c, m))?;
        let mut sh = sh.borrow_mut();
        let colour = sh.seats.colour(p);
        *SeatState::slot(&mut sh.seats.names, p) = Some(name.clone());
        sh.cmds.push(Cmd::Ui { body: seat_event(p, &name, colour) });
        s.clear();
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "player_colour", [sh], |c, s| {
        let p = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        if matches!(s.get(1), Value::Nil) {
            let colour = sh.borrow().seats.colour(p);
            s.replace(c, colour.map_or(Value::Nil, Value::Integer));
            return Ok(CallbackReturn::Return);
        }
        let rgb = a(c, &s, 1)?;
        if !(0..=0x00ff_ffff).contains(&rgb) {
            return Err(fail(c, format!("invalid_colour: {rgb} is outside 0..0xffffff")));
        }
        let mut sh = sh.borrow_mut();
        let name = sh.seats.name(p);
        *SeatState::slot(&mut sh.seats.colours, p) = Some(rgb);
        sh.cmds.push(Cmd::Ui { body: seat_event(p, &name, Some(rgb)) });
        s.clear();
        Ok(CallbackReturn::Return)
    });
    native!(ctx, game, "setting", [sh], |c, s| {
        let p = player_arg(a(c, &s, 0)?).map_err(|m| fail(c, m))?;
        let key = text(c, s.get(1), 1)?;
        valid_key(&key).map_err(|m| fail(c, m))?;
        if matches!(s.get(2), Value::Nil) {
            let value = sh.borrow().seats.setting(p, &key);
            s.replace(c, value.map_or(Value::Nil, Value::Integer));
            return Ok(CallbackReturn::Return);
        }
        let value = a(c, &s, 2)?;
        let mut sh = sh.borrow_mut();
        let map = SeatState::slot(&mut sh.seats.settings, p);
        if !map.contains_key(&key) && map.len() >= SETTINGS_MAX {
            return Err(fail(c, format!("invalid_setting: a player holds at most {SETTINGS_MAX} settings")));
        }
        map.insert(key.clone(), value);
        sh.cmds.push(Cmd::Ui {
            body: format!(r#""kind":"setting","player":{p},"key":{},"value":{value}"#, shared::ui_json_string(&key)),
        });
        s.clear();
        Ok(CallbackReturn::Return)
    });
}

pub(super) fn allies_of(sim: &Sim, player: u8) -> Vec<u8> {
    let Some(team) = sim.team_of(player) else { return vec![player] };
    (0..MAX_PLAYERS as u8).filter(|&q| sim.team_of(q) == Some(team)).collect()
}
