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

## Phase 3 — comrades — done

- Key files, HELLO, capabilities
- NAGRAD / OTYAT
- Signed mutations (node-signed)

**Exit:** unsigned mutation refused. (`unsigned_mutation_refused`)

## Phase 4 — bureau at 25 — done

- Gulag ration
- Partial results + retry
- Review delay
- Optional samokrit
- ACCUSE / CONFISKAT / OSVOBOD

**Exit:** tests in `08-bureaucracy.md` Testing contract pass.

## Phase 5 — mesh — done

- Two-node join ceremony
- ZAVERSHIT SOYUZ
- Repair
- Placement RF=2

**Exit:** write on A, OBTAN on B. (`mesh_write_a_read_b`)

## Phase 6 — hardness — done

- Fuzz parser and WAL
- `deny.toml` + parser/WAL fuzz
- Encryption at rest (XChaCha20-Poly1305 16KiB pages)
- CHEKA expiry enforced
- Driver OCHERED/1 + `oursqld --admin`

## Phase 7 — bilets + in-situ — done

- Capability is a real struct: bilet / comrade / deystv / predel / srok / komitet / uslov
- `NAGRAD ... PREDEL ... SROK`, `POKAZ BILET`
- NEED/SNAPSHOT repair at boot
- 4-plant in-process + 4+1 process in-situ tests

## Phase 8 — half-built closed — done

- B+tree pager + buffer pool + group commit + wrapped kollektiv key
- WAL commit digest/sig, CONFISKAT TTL, durable audit
- BRIGADE / PRIOKAZ / LEVSOYUZ / YEDINSTVO / OBYCHNO / SOLIDARITY
- PETITION, ZAPOR, VIZOR, OCHERED, HELLO KEY/PODPIS
- plan_id loan/repay, approval at 60, REVIEW_WAIT, node.toml
- CI workflow + grammar.md

## Phase 9 — charter true — done

- ZAVERSHIT SOYUZ waits for quorum (`2102` if a peer is down; local row stays)
- oursqld speaks OCHERED/1 when the first byte is a frame; T_BIND + `$1`
- WAL commit digest+podpis verified on reopen
- HELLO KEY/PODPIS required once a comrade has a key
- RAZBOR prints NARODKEY / SPRAVKA / SEQSCAN; GIVEN uses the pager
- B+tree insert/split/delete in place; checkpoint is a flush

## Phase 10 — mesh matches the docs — done

- `USTANOV rf = N` places APPLY on `owners(narodkey, N)` (0 = everyone)
- `NAGRAD SOYUZ NA COMRADE plant` / `LEAVE COMRADE plant` bump the view epoch
- Komitet is a real set; only members may NAGRAD (`2111 NOT_KOMITET`)
- `NAGRAD ... RATION n MAXROWS n SAMOKRIT` is enforced
- `kill -9` after ZAVERSHIT: rows survive reopen
- Edition 2024; CI `cargo deny check` is no longer `|| true`

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
