# 03 — Architecture

## Layers

```
+------------------------------------------------------+
|  Comrade client (CLI, app driver)                    |
|  NashCQL text  |  binary envelopes                   |
+----------------+-------------------------------------+
|  Wire (oursql-wire)  mTLS, framing, backpressure     |
+------------------------------------------------------+
|  Session (dossier, ration counters, intensity view)  |
+----------------+------------------+------------------+
|  Bureau        |  NashCQL front   |  Capability gate |
|  (policy)      |  parse/plan/exec |  (authz)         |
+----------------+------------------+------------------+
|  Consensus / mesh (oursql-consensus)                 |
|  views, certification, placement                     |
+------------------------------------------------------+
|  Storage engine (oursql-storage)                     |
|  encrypted pages, WAL, indexes                       |
+------------------------------------------------------+
```

Each box is a crate. Cyclic deps are forbidden. Bureau may *observe*
planner estimates; it may not rewrite page layouts.

## Request path (OBTAN)

1. Frame decoded. Session dossier loaded.
2. Bureau: ration check. Too many requests -> error `1905 GULAG`.
3. Lexer + parser -> IR.
4. Capability gate: can this comrade OBTAN these columns?
5. Planner produces a plan. Indexes are allowed. Equality is not a crime.
6. Bureau (intensity 25): with small probability, mark some qualifying
   page-ids as `LOANED` and return `1902 COLLECTIVE_PARTIAL`.
7. Executor pulls rows from storage.
8. Response includes `dossier`, `plan_id`, and any notices.

## Mutation path (INZRT / OPDAT / REMOV / DDL)

1. Same as 1-4 above.
2. Canonicalize statement. Hash. Client or node signs (two modes;
   see [07](07-comrades-and-auth.md)).
3. Bureau: optional samokrit, optional review delay, optional approval.
4. Storage applies in a local transaction, WAL fsync if `ZAVERSHIT LOCAL`.
5. Consensus broadcasts digest if `ZAVERSHIT SOYUZ`.
6. Audit row appended (always).

## Processes

A node is one OS process (`oursqld`) plus optional sidecar CLI.

- **Listener** — accepts mTLS.
- **Session workers** — async tasks (tokio), not unbounded OS threads
  per socket. Hard cap in config.
- **Page cleaner** — WAL checkpoint, compaction.
- **Mesh** — gossip + certification.
- **Bureau clerk** — timers for delays, gulag release, confiscation expiry.

## Data objects

| Object | Owner | Durable |
| --- | --- | --- |
| Page (16 KiB) | storage | yes, encrypted |
| WAL segment | storage | yes |
| Comrade record | authz | yes |
| Capability | authz | yes, expires |
| Mesh view | consensus | yes (epoch) |
| Bureau timers | bureau | in-memory + WAL for holds |

## Single-node vs mesh

A lonely node is a valid kollektiv of size 1. `ZAVERSHIT SOYUZ` on a
size-1 view is local commit. Adding a second host is a membership
ceremony, not a rewrite of user tabls.

## Why this shape

- **Separation** keeps satire from corrupting durability.
- **Signed IR** keeps two dialects from becoming two security holes.
- **Async workers** keep the "complicated" part in policy, not in
  thread-per-connection collapse.
- **Host-local pages** keep the means of hosting with the operator.
