# Server log schema

Every line the server prints, on stdout or stderr, is `ev=<name>` followed by `key=value` fields in call-site order. The grammar lives in `rust/crates/server/src/log.rs`: a bare value holds no whitespace, quote, backslash, `=` or control character; any other value is quoted with `"` and `\` escaped.

This file is the schema. It has one row per event name in `log.rs`. The CI job `log-schema` reads this table and checks a server log against it: every line must parse, its name must be documented, its keys must be listed for that name, and every event marked `required` must appear in the run.

The `tick_time` keys `system` and `samples` appear only on the optional per-system lines (one `ev=tick_time system=<name>` per sim system); the total line carries the other keys.

Columns: `Event` is the `ev=` name, `Variant` the `Ev` variant, `Emitted from` the source file under `rust/crates/server/src`, `Fields` the keys in call-site order (`-` for none), `Sample` is `required` when the `server-log-grammar` driver always produces the event, `optional` otherwise.

| Event | Variant | Emitted from | Fields | Sample |
|---|---|---|---|---|
| `serving` | Serving | lib.rs | rooms, addr, rate | required |
| `fatal` | Fatal | main.rs | error | optional |
| `accept_error` | AcceptError | lib.rs | error | optional |
| `mod_loaded` | ModLoaded | lib.rs | path, version | required |
| `order_token` | OrderToken | lib.rs | room, token | required |
| `tick_time` | TickTime | budget.rs | ticks, p50_us, p90_us, p99_us, max_us, queued, overruns, system, samples | required |
| `tick_overrun` | TickOverrun | room.rs | room, tick, cost_us, budget_us | required |
| `tick_shed` | TickShed | room.rs | room, tick, shed, cap | optional |
| `tick_shed_edge` | TickShedEdge | room.rs | room, tick, state | optional |
| `mod_cmds` | ModCmds | room.rs | room, tick, applied | optional |
| `mod_cmds_dropped` | ModCmdsDropped | mods.rs | count | optional |
| `mod_unit_bound` | ModUnitBound | room.rs | handle, unit | optional |
| `mod_unit_refused` | ModUnitRefused | room.rs | handle, reason | optional |
| `mod_reload_staged` | ModReloadStaged | mods.rs | - | optional |
| `mod_reload_failed` | ModReloadFailed | mods.rs | error | required |
| `mod_image_reloaded` | ModImageReloaded | mods.rs | bytes | optional |
| `mod_image_reload_failed` | ModImageReloadFailed | mods.rs | error | optional |
| `data_reloaded` | DataReloaded | room.rs | room, tick, units, carried, refused | optional |
| `data_reload_failed` | DataReloadFailed | mods.rs | room, error | optional |
| `mod_error` | ModError | mods.rs | tick, error | required |
| `mod_tick` | ModTick | mods.rs | tick, version, probe | required |
| `client_error` | ClientError | relay.rs | session, tick, tokens, message | required |
| `client_error_dropped` | ClientErrorDropped | relay.rs | session, count | optional |
| `client_error_log_failed` | ClientErrorLogFailed | relay.rs | op, path, error | optional |
| `ticker_died` | TickerDied | supervise.rs | thread, reason | optional |
| `ticker_restarted` | TickerRestarted | supervise.rs | thread, restarts | optional |
| `mod_thread_died` | ModThreadDied | supervise.rs | thread, reason | optional |
| `mod_thread_restarted` | ModThreadRestarted | supervise.rs | thread, restarts | optional |
| `supervisor_giveup` | SupervisorGiveup | supervise.rs | thread, restarts | optional |
| `conn_refused` | ConnRefused | lib.rs | limit | optional |