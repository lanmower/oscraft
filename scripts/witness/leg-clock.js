(() => {
  window.legClock = {
    create(startTick, legTicks, capTicks) {
      let last = startTick;
      let spent = 0;
      return {
        advance(tick, counts) {
          if (counts && tick > last) spent += tick - last;
          last = tick;
          return spent < legTicks && tick - startTick < capTicks;
        },
      };
    },
  };
})();
