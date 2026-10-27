/* tslint:disable */
/* eslint-disable */

export class Game {
    free(): void;
    [Symbol.dispose](): void;
    abilities_json(unit: number): string;
    ability_cooldown(unit: number, slot: number): number;
    attack(unit: number, target: number): boolean;
    attack_move(unit: number, x: number, y: number): boolean;
    cast(unit: number, slot: number, target: number, x: number, y: number): boolean;
    class_override(unit: number): number;
    day_cycle(): Uint32Array;
    dead_props(): Uint32Array;
    deaths_of(unit: number): number;
    dispatch_action(name: string, args: string): string;
    drain_effects(): string;
    drain_kill_events(): Uint32Array;
    drain_ui(): string;
    fog_on(): boolean;
    give_item(unit: number, item: number, count: number): boolean;
    gold(player: number): number;
    ground_items(): Float32Array;
    hold(unit: number): boolean;
    hold_override(unit: number): number;
    hostile(a: number, b: number): boolean;
    hurt(unit: number, damage: number): void;
    inventory_json(unit: number): string;
    invisible_left(unit: number): number;
    item_catalog_json(): string;
    kills_of(unit: number): number;
    last_refusal(): string;
    load_mods(json: string): string;
    load_state(bytes: Uint8Array): void;
    map_size(): Uint32Array;
    constructor(image: Uint8Array);
    order_move(unit: number, x: number, y: number): boolean;
    order_queue_clear(unit: number): boolean;
    order_queue_push(unit: number, kind: number, x: number, y: number, target: number): boolean;
    pointer(kind: string, x: number, y: number, button: number): string;
    remove_unit(unit: number): boolean;
    replay_end(): number;
    replay_incomplete(): number;
    replay_load(text: string): number;
    replay_seek(tick: number): number;
    replay_step(n: number): number;
    root_left(unit: number): number;
    save_state(): Uint8Array;
    set_attr_rules(text: string): string;
    set_class_override(unit: number, _class: number): boolean;
    set_fog(on: boolean): void;
    set_hold_override(unit: number, hold: number): boolean;
    silence_left(unit: number): number;
    skill_up(unit: number, slot: number): boolean;
    slow_left(unit: number): number;
    slow_multiplier(unit: number): number;
    spawn_attributed(player: number, kind: number, x: number, y: number): number;
    state_hash(): bigint;
    status_left(unit: number, kind: number): number;
    stun_left(unit: number): number;
    swap_data(image: Uint8Array): Uint32Array;
    tick(): string;
    ticks(): number;
    unit_attacking(): Uint8Array;
    unit_filter_bits(): Uint32Array;
    unit_flags(): Uint8Array;
    unit_gold(unit: number): number;
    unit_headings(): Int32Array;
    unit_hp(unit: number): number;
    unit_hps(): Int32Array;
    unit_ids(): Uint32Array;
    unit_item_gen(): Uint32Array;
    unit_max_hps(): Int32Array;
    unit_model_names(): string[];
    unit_periods(): Int32Array;
    unit_scales(): Uint16Array;
    unit_strike_target(): Int32Array;
    unit_type_names(): string[];
    unit_types(): Uint32Array;
    units(): Float32Array;
    use_item(unit: number, slot: number, target: number, x: number, y: number): boolean;
    vision(p: number): Uint8Array;
    vision_dims(): Uint32Array;
    vision_generation(): number;
    winner(): number;
    xp(unit: number): number;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_game_free: (a: number, b: number) => void;
    readonly game_abilities_json: (a: number, b: number) => [number, number];
    readonly game_ability_cooldown: (a: number, b: number, c: number) => number;
    readonly game_attack: (a: number, b: number, c: number) => number;
    readonly game_attack_move: (a: number, b: number, c: number, d: number) => number;
    readonly game_cast: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly game_class_override: (a: number, b: number) => number;
    readonly game_day_cycle: (a: number) => [number, number];
    readonly game_dead_props: (a: number) => [number, number];
    readonly game_deaths_of: (a: number, b: number) => number;
    readonly game_dispatch_action: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly game_drain_effects: (a: number) => [number, number];
    readonly game_drain_kill_events: (a: number) => [number, number];
    readonly game_drain_ui: (a: number) => [number, number];
    readonly game_fog_on: (a: number) => number;
    readonly game_give_item: (a: number, b: number, c: number, d: number) => number;
    readonly game_gold: (a: number, b: number) => number;
    readonly game_ground_items: (a: number) => [number, number];
    readonly game_hold: (a: number, b: number) => number;
    readonly game_hold_override: (a: number, b: number) => number;
    readonly game_hostile: (a: number, b: number, c: number) => number;
    readonly game_hurt: (a: number, b: number, c: number) => void;
    readonly game_inventory_json: (a: number, b: number) => [number, number];
    readonly game_invisible_left: (a: number, b: number) => number;
    readonly game_item_catalog_json: (a: number) => [number, number];
    readonly game_kills_of: (a: number, b: number) => number;
    readonly game_last_refusal: (a: number) => [number, number];
    readonly game_load_mods: (a: number, b: number, c: number) => [number, number];
    readonly game_load_state: (a: number, b: number, c: number) => [number, number];
    readonly game_map_size: (a: number) => [number, number];
    readonly game_new: (a: number, b: number) => [number, number, number];
    readonly game_order_move: (a: number, b: number, c: number, d: number) => number;
    readonly game_order_queue_clear: (a: number, b: number) => number;
    readonly game_order_queue_push: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly game_pointer: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly game_remove_unit: (a: number, b: number) => number;
    readonly game_replay_end: (a: number) => number;
    readonly game_replay_incomplete: (a: number) => number;
    readonly game_replay_load: (a: number, b: number, c: number) => [number, number, number];
    readonly game_replay_seek: (a: number, b: number) => [number, number, number];
    readonly game_replay_step: (a: number, b: number) => [number, number, number];
    readonly game_root_left: (a: number, b: number) => number;
    readonly game_save_state: (a: number) => [number, number];
    readonly game_set_attr_rules: (a: number, b: number, c: number) => [number, number];
    readonly game_set_class_override: (a: number, b: number, c: number) => number;
    readonly game_set_fog: (a: number, b: number) => void;
    readonly game_set_hold_override: (a: number, b: number, c: number) => number;
    readonly game_silence_left: (a: number, b: number) => number;
    readonly game_skill_up: (a: number, b: number, c: number) => number;
    readonly game_slow_left: (a: number, b: number) => number;
    readonly game_slow_multiplier: (a: number, b: number) => number;
    readonly game_spawn_attributed: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly game_state_hash: (a: number) => bigint;
    readonly game_status_left: (a: number, b: number, c: number) => number;
    readonly game_stun_left: (a: number, b: number) => number;
    readonly game_swap_data: (a: number, b: number, c: number) => [number, number, number, number];
    readonly game_tick: (a: number) => [number, number];
    readonly game_ticks: (a: number) => number;
    readonly game_unit_attacking: (a: number) => [number, number];
    readonly game_unit_filter_bits: (a: number) => [number, number];
    readonly game_unit_flags: (a: number) => [number, number];
    readonly game_unit_gold: (a: number, b: number) => number;
    readonly game_unit_headings: (a: number) => [number, number];
    readonly game_unit_hp: (a: number, b: number) => number;
    readonly game_unit_hps: (a: number) => [number, number];
    readonly game_unit_ids: (a: number) => [number, number];
    readonly game_unit_item_gen: (a: number) => [number, number];
    readonly game_unit_max_hps: (a: number) => [number, number];
    readonly game_unit_model_names: (a: number) => [number, number];
    readonly game_unit_periods: (a: number) => [number, number];
    readonly game_unit_scales: (a: number) => [number, number];
    readonly game_unit_strike_target: (a: number) => [number, number];
    readonly game_unit_type_names: (a: number) => [number, number];
    readonly game_unit_types: (a: number) => [number, number];
    readonly game_units: (a: number) => [number, number];
    readonly game_use_item: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly game_vision: (a: number, b: number) => [number, number];
    readonly game_vision_dims: (a: number) => [number, number];
    readonly game_vision_generation: (a: number) => number;
    readonly game_winner: (a: number) => number;
    readonly game_xp: (a: number, b: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __externref_drop_slice: (a: number, b: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
