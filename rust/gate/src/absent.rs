//! Checks whose API is not on the sim or mod-host public surface. Each names the API it needs,
//! so its capability row stays unverified until that API is public.

use crate::checks::missing;

pub fn event_delivery(_image: &[u8]) -> Result<(), String> {
    missing("mod-host attack order: order_unit moves only, so on_attacked and on_died cannot fire from a script")
}
