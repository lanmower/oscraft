# Browser witnesses

Three witnesses remain scoped to a dev origin: HMR, finished match, and two-tab dispatch. Each is one bounded playwriter run.

Open pages in the worker own context; keep at most two pages open at a time.
Before each step, list the pages and act only on pages you opened yourself.
Close every page and stop every server in a finally block; name every page and server you closed in the report. Never reuse a page from the shared list.
A stale .dev-site keeps a bot-filled host at tick 0, so re-stage from HEAD with SKIP_PACKS=1 before a witness.
Each witness records its evidence in DESIGN.md. No witness is a CI gate.
