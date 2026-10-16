/* tslint:disable */
/* eslint-disable */

export class Game {
    free(): void;
    [Symbol.dispose](): void;
    constructor(image: Uint8Array);
    /**
     * Orders unit `unit` to walk to tile (x, y). Returns false when the sim refuses the order.
     */
    order_move(unit: number, x: number, y: number): boolean;
    state_hash(): bigint;
    tick(): void;
    ticks(): number;
    /**
     * Sim ids of the living units, in the same order as `units()`.
     */
    unit_ids(): Uint32Array;
    /**
     * Living units as flat `[x, y, player]` triples, positions in tiles.
     */
    units(): Float32Array;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_game_free: (a: number, b: number) => void;
    readonly game_new: (a: number, b: number) => [number, number, number];
    readonly game_order_move: (a: number, b: number, c: number, d: number) => number;
    readonly game_state_hash: (a: number) => bigint;
    readonly game_tick: (a: number) => void;
    readonly game_ticks: (a: number) => number;
    readonly game_unit_ids: (a: number) => [number, number];
    readonly game_units: (a: number) => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
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
