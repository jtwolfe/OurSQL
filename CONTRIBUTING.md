# Contributing (a petition)

Patches are welcome. They are not owed merge.

## How

1. Open a discussion or issue describing the change in engineering terms.
2. Fork. Branch name: `petition/<short-ascii>`.
3. `cargo fmt` and `cargo test` (when the tree compiles more than stubs).
4. Pull request. Title in ASCII. Body includes:
   - which doc section you are implementing or amending
   - intensity impact (must not break the intensity-0 usefulness bar)
   - new error codes if any

## Rules

- US-keyboard ASCII in identifiers, keywords, and official docs.
- No `unsafe` without an ID in `docs/09-security.md`.
- Bureau must not depend on storage.
- Do not add a coin.
- Do not add cyrillic "for flavor."
- Satire stays in bureau and docs. Storage stays boring.

## Code of tone

Be sharp. Do not be cruel to people. The oppression is a specified
software module, not a social license.
