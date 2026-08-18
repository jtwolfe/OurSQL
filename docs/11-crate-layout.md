# 11 -- Crate layout

Workspace root `Cargo.toml` is virtual.

```
oursql/
  crates/
    oursql-core         # types, ids, errors, intensity
    oursql-crypto       # thin wrappers, zeroize
    oursql-storage      # SKLAD
    oursql-nashcql      # lexer, parser, IR
    oursql-engine       # planner + executor
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
core < core < nashcql
core < bureau
core < authz
core < consensus
core < wire
engine depends on storage + nashcql + bureau + authz + consensus + crypto
node depends on engine + wire + consensus
cli depends on engine (embedded) and can talk OCHERED/1 via driver
```

`oursql-bureau` MUST NOT depend on `oursql-storage`.

## oursql-core responsibilities

- `Intensity(u8)` newtype, clamps 0..=100
- `Dossier`, `ComradeId`, `NodeId`, `NarodKey`
- `Error` with stable codes
- Canonical encoding for signatures (length-prefixed, no serde surprises)

## Edition and lints

- edition = "2024", rust-version 1.85
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
