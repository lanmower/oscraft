# Server log schema

The HTTP server crate (`rust/crates/server`), its `ev=` log grammar and the CI job that validated server logs are not part of this repository. The browser page is the host ([HOST.md](HOST.md)); there is no server log schema to maintain.

The retired event table is kept in this file's git history (`git log -- docs/LOG_SCHEMA.md`, then `git show <commit>:docs/LOG_SCHEMA.md`). The `oscraft-gate` check names in docs/CAPABILITIES.md are historical labels for the same reason: `rust/gate` is not a workspace member.
