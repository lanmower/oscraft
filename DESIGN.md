# Design notes

## 2026-11-05

- gm-continue setup: `tools/gm-continue-setup.mjs` was missing from the checkout, so `node tools/gm-continue-setup.mjs` failed with `Cannot find module`. Restored it as a read-only script: it reads `work/rows/*.yml`, prints each open row as a plan line, reports rows whose status is neither open nor resolved as defects, and writes nothing. DESIGN.md did not exist in this checkout, so this file was created to hold the note.
