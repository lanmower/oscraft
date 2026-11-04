export function createModelOrder() {
  let selectedPending = 0;
  let selectedIdle = Promise.resolve();
  let markSelectedIdle = () => {};
  const speculativeFetches = new Set();

  function selected(load) {
    if (selectedPending === 0) {
      selectedIdle = new Promise((resolve) => { markSelectedIdle = resolve; });
      for (const controller of speculativeFetches) controller.abort();
    }
    selectedPending += 1;
    return load(null).finally(() => {
      selectedPending -= 1;
      if (selectedPending === 0) markSelectedIdle();
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

  return { selected, speculative };
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
