# 04 edit: publish-on-origin-partial-clone-write-tree
session: gm-drain-1793526214-19

no edit to /workspace/oscraft source. The fix (write-tree --missing-ok in publish_on_origin) is already on gm-mcp origin/main at 3df5e2237d991656697621efaf7669836fa1b24b, which is a separate repository (AnEntrypoint/gm-mcp), reached through the gm checkout at /workspace/gm/gm-mcp. GIT_NO_LAZY_FETCH=1 is not added: 03-eliminate shows it has no effect on write-tree.

references recorded: https://github.com/AnEntrypoint/gm-mcp/commit/3df5e2237d991656697621efaf7669836fa1b24b (dispatch 1793528338017-824759-65fddebc614b66c6).

closing publish: the row file .gm/prd/publish-on-origin-partial-clone-write-tree.yml is removed and closes/publish-on-origin-partial-clone-write-tree.txt is added by publish_on_origin; the witness is the verifier lane's dispatch.
