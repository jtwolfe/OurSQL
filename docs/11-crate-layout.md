# 11 -- Crate layout

Workspace root `Cargo.toml` is virtual.

```
oursql/
  crates/
    oursql-core         # types, ids, errors, intensity
    oursql-crypto       # BLAKE3, Ed25519, XChaCha20, CRC
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
core < nashcql
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
- `unsafe_code` deny (workspace lint). No `missing_docs` lint yet.

## Binaries

| Bin | Crate | Job |
| --- | --- | --- |
| oursqld | oursql-node | server |
| oursql | oursql-cli | REPL + admin |

Each crate exports `version()`. The doc check in
`oursql-engine/tests/doc_check.rs` keeps the catalog, keywords, and
OCHERED types from drifting.
