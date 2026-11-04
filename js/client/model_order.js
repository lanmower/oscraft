export function createModelOrder() {
  let selectedPending = 0;
  let idle = true;
  let selectedIdle = Promise.resolve();
  let markSelectedIdle = () => {};
  const speculativeFetches = new Set();

  function markIdle() {
    if (selectedPending > 0 || idle) return;
    idle = true;
    markSelectedIdle();
  }

  function selected(load) {
    if (selectedPending === 0) {
      if (idle) {
        idle = false;
        selectedIdle = new Promise((resolve) => { markSelectedIdle = resolve; });
      }
      for (const controller of speculativeFetches) controller.abort();
    }
    selectedPending += 1;
    return load(null).finally(() => {
      selectedPending -= 1;
      if (selectedPending === 0) setTimeout(markIdle, 0);
    });
  }

  async function speculative(ready, load) {
    for (;;) {
      await ready;
      while (selectedPending > 0) await selectedIdle;
      const controller = new AbortController();
      speculativeFetches.add(controller);
      try {
        return await untilYielded(load(controller.signal), controller.signal);
      } catch (err) {
        if (!controller.signal.aborted) throw err;
      } finally {
        speculativeFetches.delete(controller);
      }
    }
  }

  function ordered(isSelected, ready, load) {
    return isSelected ? selected(load) : speculative(ready, load);
  }

  return { selected, speculative, ordered };
}

function untilYielded(loading, signal) {
  return new Promise((resolve, reject) => {
    const yielded = () => reject(new Error("speculative load yielded to a selected model"));
    signal.addEventListener("abort", yielded, { once: true });
    loading.then(
      (value) => { signal.removeEventListener("abort", yielded); resolve(value); },
      (err) => { signal.removeEventListener("abort", yielded); reject(err); },
    );
  });
}
