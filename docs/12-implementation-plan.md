# 12 — Implementation plan

## Phase 0 — this repository

- Charter and design docs (done)
- Workspace skeleton, error codes, intensity type
- Example NashCQL file
- CI later: `cargo fmt`, `cargo test`, `cargo deny` when available

## Phase 1 — single-node SKLAD

- Page file, WAL, NARODKEY tabl
- NACHAT / ZAVERSHIT LOCAL / OTMENA
- Crash-recovery test (kill after fsync)
- No mesh, no bureau

**Exit:** INZRT + OBTAN survive kill -9.

## Phase 2 — NashCQL front

- Lexer + parser for the keyword table
- Decadent SQL rewrite
- Planner: seq scan + narodkey point lookup
- CLI REPL

**Exit:** examples/hello-kollektiv.nql runs.

## Phase 3 — comrades

- Key files, HELLO, capabilities
- NAGRAD / OTYAT
- Signed mutations (node-signed)

**Exit:** unsigned mutation refused.

## Phase 4 — bureau at 25

- Gulag ration
- Partial results + retry
- Review delay
- Optional samokrit
- ACCUSE / CONFISKAT / OSVOBOD

**Exit:** tests in `08-bureaucracy.md` Testing contract pass.

## Phase 5 — mesh

- Two-node join ceremony
- ZAVERSHIT SOYUZ
- Repair
- Placement RF=2

**Exit:** write on A, OBTAN on B.

## Phase 6 — hardness

- Fuzz parser and WAL
- cargo deny
- Encryption at rest on by default
- CHEKA expiry enforced
- Driver crate + a tiny HTTP admin (optional)

## Explicitly later

- Stored procedures
- Full SQL compatibility pack
- Geographic pin UI
- Any coin

## Suggested first code files

1. `crates/oursql-core/src/intensity.rs`
2. `crates/oursql-core/src/error.rs`
3. `crates/oursql-nashcql/src/keywords.rs`
4. `crates/oursql-storage/src/wal.rs`
5. `crates/oursql-bureau/src/ration.rs`
