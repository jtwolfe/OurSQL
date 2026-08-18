# 04 -- Storage engine

Codename: **SKLAD**.

SKLAD is a page engine, not a heap-plus-sidecar-index toy.

## Pages

- Size: 16 KiB (configurable 8/16/32 at kollektiv create, then frozen).
- Checksum: BLAKE3 over plaintext, stored in the encrypted header.
- Encryption: XChaCha20-Poly1305 per page. Key is the kollektiv data key,
  wrapped per node with the node's storage key.
- Page types: leaf, branch, overflow, undo, meta, freelist.

## Indexes

Tabls are **index-organized** on `NARODKEY` (the collective primary key).
Secondary indexes (`SPRAVKA`) store the secondary key plus the narodkey.
Lookups that are not covering do a second walk ("bookmark").

This is boring and correct. We do not invent a new tree.

## WAL

- Redo-before-write.
- Group commit.
- `ZAVERSHIT LOCAL` = WAL fsync of the mutation + commit record.
- Undo chain exists for rollback (`OTMENA`) and for snapshot OBTAN.

Crash recovery: replay redo, roll back loser transactions, drop in-flight
bureau timers except confiscation holds (those are WAL'd).

## Encryption at rest

- Node storage key: operator-provided or generated at init, never logged.
- Kollektiv data key: created at MANUFAKTUR KOLLEKTIV, wrapped to each
  member node's storage key during the join ceremony.
- Leaving the mesh: local pages remain readable to that operator. Other
  nodes keep their copies. There is no remote wipe command. CONFISKAT is
  a logical hold, not a crypto-shred.

## Files on disk

```
$OURL_DATA/
  node.key                 (mode 0600)
  authz.json               (bilets)
  view.json                (members, epoch, rf, default_commit)
  wal/000000.log           (legacy: wal.log at the data root)
  kollektiv/sklad/meta     (wrapped data key)
  kollektiv/sklad/pages/tree.pg
  kollektiv/sklad/audit/audit.log
  kollektiv/pages/checkpoint.pg
```

`node.toml` is not written here. Pass `--config` if you want one.

No world-readable defaults. Init refuses to start if `node.key` is
group-readable.

## What we refuse

- Memory engine as default (data must survive the process).
- "Just trust the filesystem" without checksums.
- Compression that is on by default before we have a fuzzer.
- Cross-kollektiv page sharing.

## Intensity interaction

None. SKLAD does not know the word gulag. Bureau may *delay* calling
into SKLAD. It may not skip WAL.
