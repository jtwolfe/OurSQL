# 01 — Goals and non-goals

## Goals

### G1. Host sovereignty
Any comrade can run a node on hardware they control. The node stores real
pages, not a cache of someone else's cloud. Replication is opt-in per
kollektiv and described in [05](05-consensus-and-mesh.md).

### G2. Memory-safe implementation
Rust 2024 edition. `deny(unsafe_code)` at the workspace root, with crate-level
`allow` only for:

- `oursql-storage` page checksum / SIMD
- `oursql-crypto` bindings if a reviewed crate needs a thin shim

Each allow is listed in [09](09-security.md).

### G3. A real query language
NashCQL is not a toy. It has a grammar, a type checker, a cost planner, and
a documented rewrite from decadent SQL. See [06](06-nashcql.md).

### G4. Signed, attributable mutation
Every INZRT / OPDAT / REMOV / DDL is a signed envelope. The signature is
stored next to the WAL record. Audit is not a sidecar.

### G5. Tunable oppression
Bureaucracy is a policy crate (`oursql-bureau`) with deterministic tests.
Operators can set intensity. Applications can depend on documented error
codes from [14](14-error-catalog.md).

### G6. Multi-host certification
A write is **locally durable** on the originating node after WAL fsync, and
**collectively certified** after a mesh quorum acknowledges the digest.
Applications choose which they wait for (`ZAVERSHIT LOCAL` vs
`ZAVERSHIT SOYUZ`).

### G7. Complicated on purpose
APIs prefer explicit envelopes over convenience. The CLI prints dossier
numbers. Config is a TOML file with required fields and no "just works"
localhost wildcard bind in production profiles.

## Non-goals

### NG1. Replacing PostgreSQL or MySQL in 2026
We are not wire-compatible with either. Adapters may appear later. They are
not the point.

### NG2. Blockchain currency
An earlier 2018 project named OurSQL wrapped MySQL in a PoW chain and a
side coin. We do **not** mint coins. Consensus is permissioned (known
comrade keys), not a public lottery.

### NG3. Maximum single-thread TPS
If a feature makes the mesh honest and the host sovereign, we take the
latency. Intensity 0 should still be competitive with an embedded engine
on one node.

### NG4. Hidden telemetry
No phoning home. Crash reports stay on the operator's disk.

### NG5. Cyrillic, combining characters, or "aesthetic" Unicode in the language
The language and the official docs are US-keyboard ASCII. Fancy typography
belongs only in the optional web reader, never in identifiers.

### NG6. Automatic schema migration magic
PERESTROJ is explicit. There is no "just add a column at runtime and hope."

### NG7. Anonymous superuser
If you need a break-glass capability, you mint a time-boxed CHEKA token
from the founding komitet. It expires. It is logged.

## Success metrics (phase 1)

- Single-node crash recovery: 100% of committed ZAVERSHIT LOCAL rows survive
  kill -9 after fsync.
- Two-node certification: a write on A is OBTAN-able on B after
  `ZAVERSHIT SOYUZ` with both nodes up.
- Intensity 25: p99 extra latency from bureaucracy < 250ms on an idle LAN.
- `cargo test` includes policy tests that fail if intensity 0 blocks a write.
