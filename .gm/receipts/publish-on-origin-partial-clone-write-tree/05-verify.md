# 05 verify: publish-on-origin-partial-clone-write-tree
session: gm-drain-1793526214-19

verification lane (session gm-drain-1793526214-19-v) returned VERIFY: fail. Its plumbing probe was denied before running by gate bash-git-bypass (dispatch 1793528335722-4293-9004a5450e85d22e, recorded in /workspace/oscraft/.gm/exec-spool/.dispatch-ledger.json). No gm-dispatched witness of the write-tree behaviour exists.

The earlier probe in 03-eliminate.md ran as a direct Bash call in the executor session, outside a gm dispatch and with no ledger entry, so it is not a witness and is not used as one.

result: no witness, so no closing publish and no prd-resolve. Row stays pending with a blocker row filed: publish-on-origin-partial-clone-write-tree-blocker-gm-drain-1793526214-19.
