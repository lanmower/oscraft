# 03 eliminate: publish-on-origin-partial-clone-write-tree
session: gm-drain-1793526214-19

Instrument (throwaway GIT_INDEX_FILE under mktemp, removed after; shared index and working tree untouched): read-tree origin/main, then update-index --cacheinfo 100644 of a nonexistent blob for README.md, then write-tree in three forms.

result:
- plain write-tree: `error: invalid object 100644 0123456789abcdef0123456789abcdef01234567 for 'README.md'`, `fatal: git-write-tree: error building trees`, exit 128. This is the write_tree_failed condition.
- write-tree --missing-ok: tree 5bbcc6e52ef97d1b02a6c42e50fbfdbe9feedbeb, exit 0.
- GIT_NO_LAZY_FETCH=1 write-tree --missing-ok: same tree 5bbcc6e52ef97d1b02a6c42e50fbfdbe9feedbeb, exit 0.

live gm-level run: publish_on_origin dry_run on /workspace/oscraft (paths README.md, unchanged) reached write-tree and refused `nothing_to_publish` (dispatch 1793528240775-824759-3a91d103e1ac0b43), so the served verb builds the tree without write_tree_failed on the shared checkout. This does not exercise a partial clone, which the shared checkout is not.

conclusion from the instrument: the missing-ok flag is the operative fix; GIT_NO_LAZY_FETCH=1 has no effect on write-tree and is not needed for this refusal.
