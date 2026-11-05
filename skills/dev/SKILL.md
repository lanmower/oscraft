# dev

## Process lessons

- A missing tools/gm-continue-setup.mjs stops the drain before any row is worked; restore it read-only first.
- Fan out read-only scouts in small batches; gm.search under many parallel calls crashes the worker.
