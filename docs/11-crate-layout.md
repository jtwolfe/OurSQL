# 11 — Crate layout

Workspace root `Cargo.toml` is virtual.

```
oursql/
  crates/
    oursql-core         # types, ids, errors, intensity
    oursql-crypto       # thin wrappers, zeroize
    oursql-storage      # SKLAD
    oursql-nashcql      # lexer, parser, planner, exec IR
    oursql-bureau       # policy
    oursql-authz        # comrades, caps
    oursql-consensus    # mesh, certify
    oursql-wire         # framing
    oursql-node         # binary oursqld
    oursql-cli          # binary oursql
    oursql-driver       # rust client
```

## Dependency direction

```
core < crypto
core < storage
core < nashcql < bureau
core < authz
core < consensus
wire < node
node depends on all of the above
cli depends on wire + nashcql (for local parse) + driver
```

`oursql-bureau` MUST NOT depend on `oursql-storage`.

## oursql-core responsibilities

- `Intensity(u8)` newtype, clamps 0..=100
- `Dossier`, `ComradeId`, `NodeId`, `NarodKey`
- `Error` with stable codes
- Canonical encoding for signatures (length-prefixed, no serde surprises)

## Edition and lints

- edition = "2024" if the toolchain allows, else "2021"
- `rust-version` pinned in workspace
- `unsafe_code` deny
- `missing_docs` warn on published crates

## Binaries

| Bin | Crate | Job |
| --- | --- | --- |
| oursqld | oursql-node | server |
| oursql | oursql-cli | REPL + admin |

## Minimum viable compile (phase 0)

Each crate has `src/lib.rs` that exports a module map and a
`version()` fn. `oursql-core` already has `Intensity` and the error
catalog stub so docs and code cannot drift without a test.
