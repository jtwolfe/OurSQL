# 05 -- Consensus and mesh

The mesh is how **users keep the means of data hosting** without pretending
a single process is a nation.

## Membership

A **kollektiv** is a named database with:

- a founding set of node ids + comrade keys
- a replication factor `R` (`USTANOV rf`, default `0` = every plant)
- a certification quorum `Q` (majority of the placement set)

Joining:

1. Applicant presents `PETITION SOYUZ`.
2. A komitet member runs `NAGRAD SOYUZ NA COMRADE plant`.
3. The new plant `--peer`s an existing node and NEED/SNAPSHOTs.
4. View epoch increments (see `POKAZ USTANOV` / `epoch`).

Leaving is `LEAVE COMRADE plant` (or `OTYAT SOYUZ IZ COMRADE plant`).
Pages stay on that disk. You cannot LEAVE the last plant.

`USTANOV rf = N` (default `0` = everyone) places APPLY on
`owners(narodkey, N)` plus the writer. Certification quorum is majority
of that placement set. The 4-plant in-situ test leaves `rf = 0` so
every plant has every row.

## What is certified

Not every byte. Certification is over **mutation digests**:

```
digest = BLAKE3(
  kollektiv_id,
  schema_epoch,
  stmt_canonical,
  narodkeys_touched,
  comrade_id,
  ts
)
```

Nodes apply the mutation locally if:

- signature verifies
- capability was valid at `ts`
- no certification conflict on the same narodkeys in the same epoch
- bureau did not reject (approval / confiscation)

## Conflict rule

First certified digest wins. Loser receives `1912 LINE_CONFLICT` and must
OTMENA / retry. This is "first committer wins," not last-writer-wins.

## Commit kinds

| Verb | Durability | When to use |
| --- | --- | --- |
| `ZAVERSHIT LOCAL` | WAL on this host | Single-host apps, queues |
| `ZAVERSHIT SOYUZ` | WAL + quorum certify | Shared truth |
| `ZAVERSHIT CHEKA` | same as SOYUZ + extra audit | Confiscation / membership |

Clients that do not say which they want inherit `default_commit` from
`node.toml` (install default: `LOCAL` on size-1, `SOYUZ` on size>=3).

## Gossip

Nodes gossip:

- view epochs
- certified digest headers (not always full rows)
- accusation summaries
- confiscation holds
- intensity (informational; local intensity is local)

Gossip is not a query path. OBTAN always hits local SKLAD plus optional
repair.

## Repair

If B is missing a certified digest that A has, B sends `NEED` on the
mesh port. A replies `SNAPSHOT` with the exported WAL batch (every
tabl, spravka, and hold). B `apply_remote`s it.

`oursqld --peer A` does this at boot. That is how a fifth plant joins
an already-full kollektiv.

If apply fails (schema epoch mismatch), B stops accepting writes and
enters `PERESTROJ-WAIT`.

## Why not a public chain

A public PoW chain would make every INZRT expensive and would reintroduce
a currency we explicitly refuse ([01](01-goals-and-non-goals.md) NG2).
Permissioned keys match "comrades we can name."

## Failure model

- Crash-stop of a minority: SOYUZ still commits.
- Crash-stop of a majority: SOYUZ stalls; LOCAL still works on survivors.
- Byzantine node: cannot forge signatures; can withhold / equivocate.
  Equivocation (two digests, same ts, same comrade) is evidence; the node
  is marked `UNRELIABLE` and excluded from Q until a komitet vote.

## Placement

Rows are placed by `hash(narodkey) % member_count` with `R` successors.
Operators may pin a tabl to a **brigade** (subset of nodes) for data
residency. Pinning is a PERESTROJ on the tabl, certified.
