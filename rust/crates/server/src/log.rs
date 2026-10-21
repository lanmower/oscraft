//! The one grammar for every line the server prints: `ev=<name>` first, then `key=value`
//! fields in the order the call site lists them. A value is bare when it is non-empty and holds
//! no whitespace, quote, backslash, `=` or control character; otherwise it is quoted, with
//! quotes and backslashes escaped and control characters written as `\n`, `\r`, `\t` or `\u{..}`,
//! so one event is always one line.

/// Every event name the server prints. The CI grammar check reads the names from `name` below.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ev {
    Serving,
    Fatal,
    AcceptError,
    ModLoaded,
    OrderToken,
    TickTime,
    TickOverrun,
    TickShed,
    TickShedEdge,
    ModCmds,
    ModCmdsDropped,
    ModUnitBound,
    ModUnitRefused,
    ModReloadStaged,
    ModReloadFailed,
    ModImageReloaded,
    ModImageReloadFailed,
    DataReloaded,
    DataReloadFailed,
    DataStageReplaced,
    StateHashLost,
    ModError,
    ModTick,
    ClientError,
    ClientErrorDropped,
    ClientErrorLogFailed,
    TickerDied,
    TickerRestarted,
    ModThreadDied,
    ModThreadRestarted,
    SupervisorGiveup,
    ConnRefused,
}

impl Ev {
    pub fn name(self) -> &'static str {
        match self {
            Ev::Serving => "serving",
            Ev::Fatal => "fatal",
            Ev::AcceptError => "accept_error",
            Ev::ModLoaded => "mod_loaded",
            Ev::OrderToken => "order_token",
            Ev::TickTime => "tick_time",
            Ev::TickOverrun => "tick_overrun",
            Ev::TickShed => "tick_shed",
            Ev::TickShedEdge => "tick_shed_edge",
            Ev::ModCmds => "mod_cmds",
            Ev::ModCmdsDropped => "mod_cmds_dropped",
            Ev::ModUnitBound => "mod_unit_bound",
            Ev::ModUnitRefused => "mod_unit_refused",
            Ev::ModReloadStaged => "mod_reload_staged",
            Ev::ModReloadFailed => "mod_reload_failed",
            Ev::ModImageReloaded => "mod_image_reloaded",
            Ev::ModImageReloadFailed => "mod_image_reload_failed",
            Ev::DataReloaded => "data_reloaded",
            Ev::DataReloadFailed => "data_reload_failed",
            Ev::DataStageReplaced => "data_stage_replaced",
            Ev::StateHashLost => "state_hash_lost",
            Ev::ModError => "mod_error",
            Ev::ModTick => "mod_tick",
            Ev::ClientError => "client_error",
            Ev::ClientErrorDropped => "client_error_dropped",
            Ev::ClientErrorLogFailed => "client_error_log_failed",
            Ev::TickerDied => "ticker_died",
            Ev::TickerRestarted => "ticker_restarted",
            Ev::ModThreadDied => "mod_thread_died",
            Ev::ModThreadRestarted => "mod_thread_restarted",
            Ev::SupervisorGiveup => "supervisor_giveup",
            Ev::ConnRefused => "conn_refused",
        }
    }
}

pub fn quote(value: &str) -> String {
    let bare = !value.is_empty()
        && !value
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '"' | '\\' | '='));
    if bare {
        return value.to_string();
    }
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub fn line(ev: Ev, fields: &[(&str, String)]) -> String {
    let mut out = format!("ev={}", ev.name());
    for (key, value) in fields {
        out.push(' ');
        out.push_str(key);
        out.push('=');
        out.push_str(&quote(value));
    }
    out
}

#[macro_export]
macro_rules! emit {
    ($ev:expr $(, $key:ident = $value:expr)* $(,)?) => {
        println!("{}", $crate::log::line($ev, &[$((stringify!($key), ($value).to_string())),*]))
    };
}

#[macro_export]
macro_rules! emit_err {
    ($ev:expr $(, $key:ident = $value:expr)* $(,)?) => {
        eprintln!("{}", $crate::log::line($ev, &[$((stringify!($key), ($value).to_string())),*]))
    };
}
