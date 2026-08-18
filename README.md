# OurSQL (NashCQL)

**The people keep the means of data hosting.**

OurSQL is a **memory-safe Rust** database designed to run as a **mesh of
comrade-operated nodes**. There is no single landlord cloud. Each operator
hosts a slice of the collective store. The query language is **NashCQL**:
English-shaped verbs that are deliberately mistakable (`PERESTROJ`, `INZRT`,
`ZAVERSHIT`) and typeable on a US keyboard.

It is **secure by construction** and **oppressive by policy**. The engine is
correct. The bureaucracy is the product.

Default policy intensity is **25%** — usable for real work, with a thin layer
of collective friction. Intensity is a single integer `0..=100`.

> Title note: *NashCQL* is the US-keyboard rendering of "Nash SQL"
> ("our SQL"). Docs use ASCII only.

## What this is

| Claim | Meaning |
| --- | --- |
| Secure | Rust, no `unsafe` in the default path, signed writes, HELLO + bilets, encrypted pages |
| Multi-host | Users run nodes. Data is placed, replicated, and certified across the mesh |
| Oppressive | Forms, dossiers, rations, accusations, confiscation, gulag rate-limits |
| Functional | ACID-ish commits, indexes, a real planner, crash recovery — at intensity 25 |

**Phases 1–11 are implemented.** Encrypted 16KiB pages, signed mutations, NashCQL **bilets**, a B+tree pager, group commit, HELLO PODPIS, BRIGADE/PRIOKAZ
(`NAGRAD` / `PREDEL` / `SROK` / `POKAZ BILET`), `ZAVERSHIT SOYUZ` mesh, NEED/SNAPSHOT
repair, 4+1 plant in-situ test, RF placement, komitet, uslov, kill -9. Bureaucracy still defaults to **25**.

```
cargo test --workspace
cargo build --release -p oursql-cli -p oursql-node
./target/release/oursql --data /tmp/sklad --intensity 0 -f examples/hello-kollektiv.nql
bash scripts/build-all-arches.sh
```

See [`docs/15-brigades.md`](docs/15-brigades.md). Implementation plan:
[`docs/12-implementation-plan.md`](docs/12-implementation-plan.md).

## Docs

| Doc | Subject |
| --- | --- |
| [00 Charter](docs/00-charter.md) | Why this exists |
| [01 Goals](docs/01-goals-and-non-goals.md) | In and out of scope |
| [02 Threat model](docs/02-threat-model.md) | Who we distrust |
| [03 Architecture](docs/03-architecture.md) | Layers, crates, data path |
| [04 Storage](docs/04-storage-engine.md) | Pages, WAL, encryption |
| [05 Consensus and mesh](docs/05-consensus-and-mesh.md) | How hosts share the store |
| [06 NashCQL](docs/06-nashcql.md) | Language, keywords, grammar |
| [07 Comrades](docs/07-comrades-and-auth.md) | Identity, capabilities |
| [08 Bureaucracy](docs/08-bureaucracy.md) | The 25% stupid layer |
| [09 Security](docs/09-security.md) | Crypto, isolation, audit |
| [10 Wire protocol](docs/10-wire-protocol.md) | Binary + text |
| [11 Crate layout](docs/11-crate-layout.md) | Rust workspace |
| [12 Implementation plan](docs/12-implementation-plan.md) | Phased build |
| [13 Ops](docs/13-ops-and-hosting.md) | Running a node |
| [14 Errors](docs/14-error-catalog.md) | Official complaints |
| [15 Brigades](docs/15-brigades.md) | Crate segmentation |
| [16 Distro](docs/16-dist.md) | Multi-arch bins |
| [Glossary](docs/glossary.md) | Terms |

## Tiny taste

```sql
ZANIM sklad;
MANUFAKTUR TABL parts (
  id        NARODKEY,
  name      TEKST NOT NYET,
  qty       CELIY,
  SOLIDARITY (depot_id) IZ depots (id)
);

INZRT V parts (name, qty) ZNACH ('bolt', 40)
  SAMOKRIT 'serves the inventory brigade';

OBTAN name, qty IZ parts GIVEN qty > 0 LINEUP name RATION 20;
```

Standard SQL is accepted as a **decadent dialect**. The planner rewrites it
and may emit a warning: `NOTICE 1901: bourgeois keywords tolerated at intensity 25`.

## Status

Phase 0 — design frozen enough to implement. See the plan.

## License

AGPL-3.0-or-later. Improvements return to the people.
See [LICENSE](LICENSE).
