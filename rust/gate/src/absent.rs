//! Checks whose API is not on the sim or mod-host public surface, or lives in a crate the gate
//! cannot link (the server is a binary with no library target). Each names the API it needs,
//! so its capability row stays unverified until that API is public.

use crate::checks::missing;

pub fn map_bounds(_image: &[u8]) -> Result<(), String> {
    missing("Sim::clamp_to_grid (pub(crate), and no public read of a clamped order target)")
}

pub fn ability_effects(_image: &[u8]) -> Result<(), String> {
    missing("Sim energy grant (energy is pub(crate) and never granted or regenerated; only pulse_shot costs 0)")
}

pub fn unsupported_effect_refuses(_image: &[u8]) -> Result<(), String> {
    missing("rules::apply_effects (pub(crate)): no public way to build an effect outside the implemented kinds")
}

pub fn clear_buff(_image: &[u8]) -> Result<(), String> {
    missing("Sim::clear_buff (pub(crate))")
}

pub fn remove_behaviour(_image: &[u8]) -> Result<(), String> {
    missing("Sim::remove_behaviour (pub(crate)); purge also needs energy, which no public API grants")
}

pub fn order_parse(_image: &[u8]) -> Result<(), String> {
    missing("server::order::parse_order (oscraftd is a binary with no library target)")
}

pub fn order_rate_limit(_image: &[u8]) -> Result<(), String> {
    missing("server::order::{rate, authorized, take} (oscraftd is a binary with no library target)")
}

pub fn area_damage(_image: &[u8]) -> Result<(), String> {
    missing("Sim::damage_area (pub(crate))")
}

pub fn heal_units(_image: &[u8]) -> Result<(), String> {
    missing("Sim energy grant (mend_field costs 50 energy; energy is never granted, so no heal can be cast)")
}

pub fn shield_bonus(_image: &[u8]) -> Result<(), String> {
    missing("Sim::shield_bonus (pub(crate)); bulwark also needs energy")
}

pub fn armour_reduces(_image: &[u8]) -> Result<(), String> {
    missing("Sim::armour_reduced and Sim::hurt (pub(crate)): no public per-hit damage read")
}

pub fn vital_fraction(_image: &[u8]) -> Result<(), String> {
    missing("Sim::vital_fraction (pub(crate)); bulwark also needs energy")
}

pub fn event_delivery(_image: &[u8]) -> Result<(), String> {
    missing("mod-host attack order: order_unit moves only, so on_attacked and on_died cannot fire from a script")
}

pub fn http_state(_image: &[u8]) -> Result<(), String> {
    missing("server::serve_state (oscraftd is a binary with no library target)")
}

pub fn http_catalog(_image: &[u8]) -> Result<(), String> {
    missing("server::serve_catalog (oscraftd is a binary with no library target)")
}

pub fn http_order(_image: &[u8]) -> Result<(), String> {
    missing("server::serve_order (oscraftd is a binary with no library target)")
}

pub fn http_delta(_image: &[u8]) -> Result<(), String> {
    missing("server::serve_delta (oscraftd is a binary with no library target; it still returns delta_unsupported)")
}

pub fn multi_room(_image: &[u8]) -> Result<(), String> {
    missing("server::parse_args and server rooms (oscraftd is a binary with no library target)")
}

pub fn http_resources(_image: &[u8]) -> Result<(), String> {
    missing("server::resources (oscraftd is a binary with no library target)")
}

pub fn presentation_refusal(_image: &[u8]) -> Result<(), String> {
    missing("sim::Refusal variants for camera, sound, UI panels and chat (none exist yet)")
}
