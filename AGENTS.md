# Agent instructions for oscraft

## Invariants

- **6.88g takes precedence over every other source, including unknowns and 6.88w3.** When a value is
  in the 6.88g data, use it. When the 6.88g data is silent, mark the value UNVERIFIED rather than
  filling it from 6.88w3, Dota 2, or a guess. 6.88w3 is used only where 6.88g has no value and the
  user has said so.
- Unknowns are recorded as UNVERIFIED in the file that holds the value, not resolved by inference.
