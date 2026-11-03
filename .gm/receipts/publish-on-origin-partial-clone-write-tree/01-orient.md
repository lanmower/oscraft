# 01 orient: publish-on-origin-partial-clone-write-tree
session: gm-drain-1793526214-19

acceptance (row text): publish_on_origin returns write_tree_failed in a blob:none partial sparse clone; fix = build the temp-index tree with `git write-tree --missing-ok` and GIT_NO_LAZY_FETCH=1.

cited code:
- publish_on_origin is a gm verb, not in this repo: /workspace/gm/gm-mcp/src/triage-verbs.js (publishOnOrigin, line ~572 onward).
- temp index read-tree origin/main: triage-verbs.js:842.
- write-tree with --missing-ok and refusal write_tree_failed: triage-verbs.js:938-940.
- the --missing-ok change is commit 3df5e2237d991656697621efaf7669836fa1b24b on gm-mcp origin/main (AnEntrypoint/gm-mcp).
- GIT_NO_LAZY_FETCH: not referenced anywhere in gm-mcp src or bin.

facts relied on:
- the shared checkout /workspace/oscraft has no remote.origin.promisor and no partialclonefilter: it is not a partial clone.
- /workspace/oscraft-fx-scratch named by the row no longer exists.
- installed gm bundle /data/cache/gm/.gm-tools/gm-mcp-server.mjs (0.2.373) contains the missing-ok fix; the served bundle reported 0.2.372 at orient time.
- codeinsight callers/impact returned zero edges for publish_on_origin (verb name is not an indexed symbol), so caller analysis is by grep of the gm-mcp source only.
- codesearch dual: bm25 channel had no publish_on_origin hits in this repo; commits channel had no write-tree fix commit (it lives in the gm-mcp submodule).
