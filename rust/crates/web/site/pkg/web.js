/* @ts-self-types="./web.d.ts" */

export class Game {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        GameFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_game_free(ptr, 0);
    }
    /**
     * @param {number} unit
     * @returns {string}
     */
    abilities_json(unit) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.game_abilities_json(this.__wbg_ptr, unit);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {number} unit
     * @param {number} slot
     * @returns {number}
     */
    ability_cooldown(unit, slot) {
        const ret = wasm.game_ability_cooldown(this.__wbg_ptr, unit, slot);
        return ret;
    }
    /**
     * @param {number} unit
     * @param {number} target
     * @returns {boolean}
     */
    attack(unit, target) {
        const ret = wasm.game_attack(this.__wbg_ptr, unit, target);
        return ret !== 0;
    }
    /**
     * @param {number} unit
     * @param {number} x
     * @param {number} y
     * @returns {boolean}
     */
    attack_move(unit, x, y) {
        const ret = wasm.game_attack_move(this.__wbg_ptr, unit, x, y);
        return ret !== 0;
    }
    /**
     * @param {number} unit
     * @param {number} slot
     * @param {number} target
     * @param {number} x
     * @param {number} y
     * @returns {boolean}
     */
    cast(unit, slot, target, x, y) {
        const ret = wasm.game_cast(this.__wbg_ptr, unit, slot, target, x, y);
        return ret !== 0;
    }
    /**
     * @param {number} unit
     * @returns {number}
     */
    class_override(unit) {
        const ret = wasm.game_class_override(this.__wbg_ptr, unit);
        return ret;
    }
    /**
     * @returns {Uint32Array}
     */
    day_cycle() {
        const ret = wasm.game_day_cycle(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {Uint32Array}
     */
    dead_props() {
        const ret = wasm.game_dead_props(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @param {number} unit
     * @returns {number}
     */
    deaths_of(unit) {
        const ret = wasm.game_deaths_of(this.__wbg_ptr, unit);
        return ret >>> 0;
    }
    /**
     * @param {string} name
     * @param {string} args
     * @returns {string}
     */
    dispatch_action(name, args) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(args, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.game_dispatch_action(this.__wbg_ptr, ptr0, len0, ptr1, len1);
            deferred3_0 = ret[0];
            deferred3_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    drain_effects() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.game_drain_effects(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {Uint32Array}
     */
    drain_kill_events() {
        const ret = wasm.game_drain_kill_events(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {string}
     */
    drain_ui() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.game_drain_ui(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {boolean}
     */
    fog_on() {
        const ret = wasm.game_fog_on(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @param {number} unit
     * @param {number} item
     * @param {number} count
     * @returns {boolean}
     */
    give_item(unit, item, count) {
        const ret = wasm.game_give_item(this.__wbg_ptr, unit, item, count);
        return ret !== 0;
    }
    /**
     * @param {number} player
     * @returns {number}
     */
    gold(player) {
        const ret = wasm.game_gold(this.__wbg_ptr, player);
        return ret;
    }
    /**
     * @returns {Float32Array}
     */
    ground_items() {
        const ret = wasm.game_ground_items(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @param {number} unit
     * @returns {boolean}
     */
    hold(unit) {
        const ret = wasm.game_hold(this.__wbg_ptr, unit);
        return ret !== 0;
    }
    /**
     * @param {number} unit
     * @returns {number}
     */
    hold_override(unit) {
        const ret = wasm.game_hold_override(this.__wbg_ptr, unit);
        return ret;
    }
    /**
     * @param {number} a
     * @param {number} b
     * @returns {boolean}
     */
    hostile(a, b) {
        const ret = wasm.game_hostile(this.__wbg_ptr, a, b);
        return ret !== 0;
    }
    /**
     * @param {number} unit
     * @param {number} damage
     */
    hurt(unit, damage) {
        wasm.game_hurt(this.__wbg_ptr, unit, damage);
    }
    /**
     * @param {number} unit
     * @returns {string}
     */
    inventory_json(unit) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.game_inventory_json(this.__wbg_ptr, unit);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {number} unit
     * @returns {number}
     */
    invisible_left(unit) {
        const ret = wasm.game_invisible_left(this.__wbg_ptr, unit);
        return ret;
    }
    /**
     * @returns {string}
     */
    item_catalog_json() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.game_item_catalog_json(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {number} unit
     * @returns {number}
     */
    kills_of(unit) {
        const ret = wasm.game_kills_of(this.__wbg_ptr, unit);
        return ret >>> 0;
    }
    /**
     * @returns {string}
     */
    last_refusal() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.game_last_refusal(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {string} json
     * @returns {string}
     */
    load_mods(json) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.game_load_mods(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {Uint8Array} bytes
     */
    load_state(bytes) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.game_load_state(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @returns {Uint32Array}
     */
    map_size() {
        const ret = wasm.game_map_size(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @param {Uint8Array} image
     */
    constructor(image) {
        const ptr0 = passArray8ToWasm0(image, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.game_new(ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        GameFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @param {number} unit
     * @param {number} x
     * @param {number} y
     * @returns {boolean}
     */
    order_move(unit, x, y) {
        const ret = wasm.game_order_move(this.__wbg_ptr, unit, x, y);
        return ret !== 0;
    }
    /**
     * @param {number} unit
     * @returns {boolean}
     */
    order_queue_clear(unit) {
        const ret = wasm.game_order_queue_clear(this.__wbg_ptr, unit);
        return ret !== 0;
    }
    /**
     * @param {number} unit
     * @param {number} kind
     * @param {number} x
     * @param {number} y
     * @param {number} target
     * @returns {boolean}
     */
    order_queue_push(unit, kind, x, y, target) {
        const ret = wasm.game_order_queue_push(this.__wbg_ptr, unit, kind, x, y, target);
        return ret !== 0;
    }
    /**
     * @param {string} kind
     * @param {number} x
     * @param {number} y
     * @param {number} button
     * @returns {string}
     */
    pointer(kind, x, y, button) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(kind, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.game_pointer(this.__wbg_ptr, ptr0, len0, x, y, button);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {number} unit
     * @returns {boolean}
     */
    remove_unit(unit) {
        const ret = wasm.game_remove_unit(this.__wbg_ptr, unit);
        return ret !== 0;
    }
    /**
     * @returns {number}
     */
    replay_end() {
        const ret = wasm.game_replay_end(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    replay_incomplete() {
        const ret = wasm.game_replay_incomplete(this.__wbg_ptr);
        return ret;
    }
    /**
     * @param {string} text
     * @returns {number}
     */
    replay_load(text) {
        const ptr0 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.game_replay_load(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * @param {number} tick
     * @returns {number}
     */
    replay_seek(tick) {
        const ret = wasm.game_replay_seek(this.__wbg_ptr, tick);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * @param {number} n
     * @returns {number}
     */
    replay_step(n) {
        const ret = wasm.game_replay_step(this.__wbg_ptr, n);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * @param {number} unit
     * @returns {number}
     */
    root_left(unit) {
        const ret = wasm.game_root_left(this.__wbg_ptr, unit);
        return ret;
    }
    /**
     * @returns {Uint8Array}
     */
    save_state() {
        const ret = wasm.game_save_state(this.__wbg_ptr);
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * @param {string} text
     * @returns {string}
     */
    set_attr_rules(text) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.game_set_attr_rules(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {number} unit
     * @param {number} _class
     * @returns {boolean}
     */
    set_class_override(unit, _class) {
        const ret = wasm.game_set_class_override(this.__wbg_ptr, unit, _class);
        return ret !== 0;
    }
    /**
     * @param {boolean} on
     */
    set_fog(on) {
        wasm.game_set_fog(this.__wbg_ptr, on);
    }
    /**
     * @param {number} unit
     * @param {number} hold
     * @returns {boolean}
     */
    set_hold_override(unit, hold) {
        const ret = wasm.game_set_hold_override(this.__wbg_ptr, unit, hold);
        return ret !== 0;
    }
    /**
     * @param {number} unit
     * @returns {number}
     */
    silence_left(unit) {
        const ret = wasm.game_silence_left(this.__wbg_ptr, unit);
        return ret;
    }
    /**
     * @param {number} unit
     * @param {number} slot
     * @returns {boolean}
     */
    skill_up(unit, slot) {
        const ret = wasm.game_skill_up(this.__wbg_ptr, unit, slot);
        return ret !== 0;
    }
    /**
     * @param {number} unit
     * @returns {number}
     */
    slow_left(unit) {
        const ret = wasm.game_slow_left(this.__wbg_ptr, unit);
        return ret;
    }
    /**
     * @param {number} unit
     * @returns {number}
     */
    slow_multiplier(unit) {
        const ret = wasm.game_slow_multiplier(this.__wbg_ptr, unit);
        return ret;
    }
    /**
     * @param {number} player
     * @param {number} kind
     * @param {number} x
     * @param {number} y
     * @returns {number}
     */
    spawn_attributed(player, kind, x, y) {
        const ret = wasm.game_spawn_attributed(this.__wbg_ptr, player, kind, x, y);
        return ret;
    }
    /**
     * @returns {bigint}
     */
    state_hash() {
        const ret = wasm.game_state_hash(this.__wbg_ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
     * @param {number} unit
     * @param {number} kind
     * @returns {number}
     */
    status_left(unit, kind) {
        const ret = wasm.game_status_left(this.__wbg_ptr, unit, kind);
        return ret;
    }
    /**
     * @param {number} unit
     * @returns {number}
     */
    stun_left(unit) {
        const ret = wasm.game_stun_left(this.__wbg_ptr, unit);
        return ret;
    }
    /**
     * @param {Uint8Array} image
     * @returns {Uint32Array}
     */
    swap_data(image) {
        const ptr0 = passArray8ToWasm0(image, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.game_swap_data(this.__wbg_ptr, ptr0, len0);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v2 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v2;
    }
    /**
     * @returns {string}
     */
    tick() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.game_tick(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {number}
     */
    ticks() {
        const ret = wasm.game_ticks(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {Uint8Array}
     */
    unit_attacking() {
        const ret = wasm.game_unit_attacking(this.__wbg_ptr);
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * @returns {Uint32Array}
     */
    unit_filter_bits() {
        const ret = wasm.game_unit_filter_bits(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {Uint8Array}
     */
    unit_flags() {
        const ret = wasm.game_unit_flags(this.__wbg_ptr);
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * @param {number} unit
     * @returns {number}
     */
    unit_gold(unit) {
        const ret = wasm.game_unit_gold(this.__wbg_ptr, unit);
        return ret;
    }
    /**
     * @returns {Int32Array}
     */
    unit_headings() {
        const ret = wasm.game_unit_headings(this.__wbg_ptr);
        var v1 = getArrayI32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @param {number} unit
     * @returns {number}
     */
    unit_hp(unit) {
        const ret = wasm.game_unit_hp(this.__wbg_ptr, unit);
        return ret;
    }
    /**
     * @returns {Int32Array}
     */
    unit_hps() {
        const ret = wasm.game_unit_hps(this.__wbg_ptr);
        var v1 = getArrayI32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {Uint32Array}
     */
    unit_ids() {
        const ret = wasm.game_unit_ids(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {Uint32Array}
     */
    unit_item_gen() {
        const ret = wasm.game_unit_item_gen(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {Int32Array}
     */
    unit_max_hps() {
        const ret = wasm.game_unit_max_hps(this.__wbg_ptr);
        var v1 = getArrayI32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {string[]}
     */
    unit_model_names() {
        const ret = wasm.game_unit_model_names(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]);
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {Int32Array}
     */
    unit_periods() {
        const ret = wasm.game_unit_periods(this.__wbg_ptr);
        var v1 = getArrayI32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {Uint16Array}
     */
    unit_scales() {
        const ret = wasm.game_unit_scales(this.__wbg_ptr);
        var v1 = getArrayU16FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 2, 2);
        return v1;
    }
    /**
     * @returns {Int32Array}
     */
    unit_strike_target() {
        const ret = wasm.game_unit_strike_target(this.__wbg_ptr);
        var v1 = getArrayI32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {string[]}
     */
    unit_type_names() {
        const ret = wasm.game_unit_type_names(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]);
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {Uint32Array}
     */
    unit_types() {
        const ret = wasm.game_unit_types(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {Float32Array}
     */
    units() {
        const ret = wasm.game_units(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @param {number} unit
     * @param {number} slot
     * @param {number} target
     * @param {number} x
     * @param {number} y
     * @returns {boolean}
     */
    use_item(unit, slot, target, x, y) {
        const ret = wasm.game_use_item(this.__wbg_ptr, unit, slot, target, x, y);
        return ret !== 0;
    }
    /**
     * @param {number} p
     * @returns {Uint8Array}
     */
    vision(p) {
        const ret = wasm.game_vision(this.__wbg_ptr, p);
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * @returns {Uint32Array}
     */
    vision_dims() {
        const ret = wasm.game_vision_dims(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {number}
     */
    vision_generation() {
        const ret = wasm.game_vision_generation(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    winner() {
        const ret = wasm.game_winner(this.__wbg_ptr);
        return ret;
    }
    /**
     * @param {number} unit
     * @returns {number}
     */
    xp(unit) {
        const ret = wasm.game_xp(this.__wbg_ptr, unit);
        return ret >>> 0;
    }
}
if (Symbol.dispose) Game.prototype[Symbol.dispose] = Game.prototype.free;
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_is_function_1f9d30630b8b1d3d: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_object_3c45d4f2dde4e749: function(arg0) {
            const val = arg0;
            const ret = typeof(val) === 'object' && val !== null;
            return ret;
        },
        __wbg___wbindgen_is_string_90b56bc79aad6f6c: function(arg0) {
            const ret = typeof(arg0) === 'string';
            return ret;
        },
        __wbg___wbindgen_is_undefined_8865fb403f8fe9d8: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_throw_41e9ee4f547fc59a: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_call_187d372bd5fdd4aa: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.call(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_crypto_38df2bab126b63dc: function(arg0) {
            const ret = arg0.crypto;
            return ret;
        },
        __wbg_getRandomValues_a608c4436c19407a: function() { return handleError(function (arg0, arg1) {
            globalThis.crypto.getRandomValues(getArrayU8FromWasm0(arg0, arg1));
        }, arguments); },
        __wbg_getRandomValues_c44a50d8cfdaebeb: function() { return handleError(function (arg0, arg1) {
            arg0.getRandomValues(arg1);
        }, arguments); },
        __wbg_length_7f3c00c40364105e: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_msCrypto_bd5a034af96bcba6: function(arg0) {
            const ret = arg0.msCrypto;
            return ret;
        },
        __wbg_new_with_length_3da0ad195f6f63ba: function(arg0) {
            const ret = new Uint8Array(arg0 >>> 0);
            return ret;
        },
        __wbg_node_84ea875411254db1: function(arg0) {
            const ret = arg0.node;
            return ret;
        },
        __wbg_process_44c7a14e11e9f69e: function(arg0) {
            const ret = arg0.process;
            return ret;
        },
        __wbg_prototypesetcall_bc27214492979395: function(arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
        },
        __wbg_randomFillSync_6c25eac9869eb53c: function() { return handleError(function (arg0, arg1) {
            arg0.randomFillSync(arg1);
        }, arguments); },
        __wbg_require_b4edbdcf3e2a1ef0: function() { return handleError(function () {
            const ret = module.require;
            return ret;
        }, arguments); },
        __wbg_static_accessor_GLOBAL_266715b9d96ba635: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_THIS_10fb7dc1ae063179: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_0b583911f537483a: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_d7f903d1508cbdc4: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_subarray_002b94d5e13d1411: function(arg0, arg1, arg2) {
            const ret = arg0.subarray(arg1 >>> 0, arg2 >>> 0);
            return ret;
        },
        __wbg_versions_276b2795b1c6a219: function(arg0) {
            const ret = arg0.versions;
            return ret;
        },
        __wbindgen_generic_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(U8)) -> NamedExternref("Uint8Array")`.
            const ret = getArrayU8FromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_generic_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./web_bg.js": import0,
    };
}

const GameFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_game_free(ptr, 1));

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

function getArrayF32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayI32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getInt32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayJsValueFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    const mem = getDataViewMemory0();
    const result = [];
    for (let i = ptr; i < ptr + 4 * len; i += 4) {
        result.push(wasm.__wbindgen_externrefs.get(mem.getUint32(i, true)));
    }
    wasm.__externref_drop_slice(ptr, len);
    return result;
}

function getArrayU16FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint16ArrayMemory0().subarray(ptr / 2, ptr / 2 + len);
}

function getArrayU32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat32ArrayMemory0 = null;
function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

let cachedInt32ArrayMemory0 = null;
function getInt32ArrayMemory0() {
    if (cachedInt32ArrayMemory0 === null || cachedInt32ArrayMemory0.byteLength === 0) {
        cachedInt32ArrayMemory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachedInt32ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint16ArrayMemory0 = null;
function getUint16ArrayMemory0() {
    if (cachedUint16ArrayMemory0 === null || cachedUint16ArrayMemory0.byteLength === 0) {
        cachedUint16ArrayMemory0 = new Uint16Array(wasm.memory.buffer);
    }
    return cachedUint16ArrayMemory0;
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedFloat32ArrayMemory0 = null;
    cachedInt32ArrayMemory0 = null;
    cachedUint16ArrayMemory0 = null;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (!module.ok) {
            throw new Error(`failed to fetch Wasm: ${module.status} ${module.statusText} fetching '${module.url}'`);
        }

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('web_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
