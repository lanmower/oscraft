const values = new Map();
export const setHot = (key, value) => { values.set(key, value); };
export const getHot = (key) => values.get(key) ?? null;
export const resetHot = () => values.clear();
