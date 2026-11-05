# Design notes

## 2026-11-05

- gm-continue setup: `tools/gm-continue-setup.mjs` was missing from the checkout, so `node tools/gm-continue-setup.mjs` failed with `Cannot find module`. Restored it as a read-only script: it reads `work/rows/*.yml`, prints each open row as a plan line, reports rows whose status is neither open nor resolved as defects, and writes nothing. DESIGN.md did not exist in this checkout, so this file was created to hold the note.
- gm worker crash: a 19-agent scout using parallel gm.search calls made the gm worker exit with code 139. Read-only investigation found unguarded shared state in SearchVerbs::searchers_ and each Searcher's files_/tracked_/index_/listing_ caches (load_tracked replaces files_ while build_index stores pointers into it, and searchers_.erase can free a Searcher in use). The gm session owns the mutex fix and concurrent search/grep regression test. This run stopped the fan-out, avoided gm.search, and used narrow gm.grep/run_shell calls.
