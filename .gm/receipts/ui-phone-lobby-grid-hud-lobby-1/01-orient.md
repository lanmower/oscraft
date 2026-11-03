# 01-orient

row: ui-phone-lobby-grid-hud-lobby-1
session: gm-drain-1793526214-15

acceptance: prd-list text (not derived). At 390x844 every hero tile inside #ui-modal; columns derive from width (auto-fill or minmax); no child right edge past the modal right edge.

## Symbols (codeinsight)
- portraitGrid js/client/widgets.js:246-285: callers 1 (buildNode widgets.js:481); callees include observeFit, badgeFontPx, actionable, setImageSrc, el.
- observeFit js/client/widgets.js:287-308: caller portraitGrid widgets.js:280.
- impact portraitGrid: 26 reached (no tests reached).

## Cited facts vs tree (origin/main, fetched)
- row cite panels.lua:1008 (cols=8 size=56) is STALE: portraits node is panels.lua:1294 `I("cols", 8), B("fit", true)`.
- widgets.js:246-251 fit mode: gridTemplateColumns = repeat(auto-fill, minmax(min(100%, px), 1fr)); non-fit keeps fixed repeat(cols, minmax(0, px)).
- widgets.js:69 .w-portraits{display:grid;max-width:100%}; widgets.js:74 .w-portraits.fit{width:100%;flex:1 1 0;min-height:0}.
- widgets.js:280 fit grids call observeFit(e, n.cols) (ResizeObserver) which sets explicit px columns that fit clientWidth.
- #ui-modal: play.html:153 body rule; modui.js:58, play.js:1569 (no width rule in the grep range).
- commit a830d270 (lanmower, Row: ui-phone-lobby-overflows) already added the fit auto-fill path on origin/main.
- file sizes match origin/main (widgets.js 25364 B, panels.lua 53251 B): local tree equals origin for these files.

## Codesearch
- dual: commits channel 2 hits (a830d270, e439fc3e); bm25 skipped (route_without_index); literal sweeps of js/client truncated at 200 files, so absence claims are not made from them.

## Open for ELIMINATE
- Whether the fit path is already sufficient at 390x844 (measured child rects vs #ui-modal right edge), i.e. whether this row is only the witness.
- Whether ui-phone-lobby-overflows (sibling, closed by a830d270?) covers this row's acceptance.
- Prior measurement 1792991970899-5485-61f184c944cf5b7d predates a830d270.

prd-list ui-phone-lobby-overflows: "dispatch_id: 1793527995109-3918-bb598c76551e9e8a\ngm_session_header_version: 1\nok: true\nverb: prd-list\nbrief: false\ncount: 1\nfolded_duplicates: 0\nfull: true\nhint: \"pass {\\\"id\\\":\\\"<row id>\\\"} for one row in full (repeated ids fold to the last); {\\\"status\\\":\\\"pending\\\"} to filter; {\\\"status\\\":\\\"pending\\\",\\\"limit\\\":<n>} to cap; {\\\"status\\\":\\\"all\\\",\\\"limit\\\":<n>,\\\"offset\\\":<n>} to page (next_offset null when done); {\\\"full\\\":true} for every full body. .gm/prd/ holds open work only; resolved rows are in .gm/prd-log.jsonl.\"\nitems[1]:\n  - de