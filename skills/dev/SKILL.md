# dev

## Process lessons

- A missing tools/gm-continue-setup.mjs stops the drain before any row is worked; restore it read-only first.
- Fan out read-only scouts in small batches; gm.search under many parallel calls crashes the worker.
- A browser-witness dev server needs a bounded lifetime: gm kills shell jobs at 10 minutes, and a stuck playwriter call can hold the whole drain.
