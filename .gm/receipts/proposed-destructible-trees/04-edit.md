# 04-edit proposed-destructible-trees

session: gm-drain-1793526214-32
edits (no code change; builds off):
1. .gm/prd/proposed-destructible-trees.yml: rescoped. The subject and description now record the refuted premise (with cites) and the four open gaps. Acceptance now requires CI-witnessed landing of each open gap or a named refusal, plus STATE_VERSION bump rules.
2. scenarios/moba/provenance/trees.lua.tsv: the slot-reuse sentence corrected to match obstacles.rs:74.
Not changed: docs/CONVERGENCE_688G.md (hy3d's doc; its line 362 still says slot reuse is missing and lists three gaps). Flagged for the owner.

State: no hashed or saved layout touched, so STATE_VERSION stays 64 and no bump is needed.
Concurrency: other lanes have dirty .gm files; this walk commits only the three explicit paths above and its receipts.
