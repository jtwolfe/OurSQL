# 13 — Ops and hosting

## The point

You run the node. You keep the disk. The mesh is a treaty, not a landlord.

## Init

```
oursqld init \
  --data /var/lib/oursql \
  --founder founder1.pub \
  --founder founder2.pub \
  --founder founder3.pub \
  --intensity 25
```

One founder is allowed. Three is the documented default.

## Run

```
oursqld run --data /var/lib/oursql --config /etc/oursql/node.toml
```

Production profile refuses:

- world-readable keys
- intensity set without a signed config event after bootstrap
- bind-any without pin-set
- missing `wal.fsync = true`

## Join a kollektiv

```
oursql petition join \
  --to nash://plant-1.example:3307 \
  --kollektiv sklad \
  --samokrit 'south depot node'
```

An existing komitet member:

```
oursql nagrad join --petition PET-... --samokrit 'accepted by founders'
```

## Backup

- Cold: stop node, copy `$OURL_DATA`.
- Hot: `oursql pokaž` is wrong (diacritics banned). Use
  `oursql pokaz backup --dest /backup/oursql`.
- Backup is pages + WAL + keys **you already have**. We do not invent a
  proprietary bundle format in phase 1; tar + checksum is enough.

## Restore

Copy data dir, start. If view epoch is ahead of local, node enters
repair.

## Intensity change

```
USTANOV kollektiv.bureau.intensity = 25 SAMOKRIT 'default restored';
```

This is a certified `_meta` write. It is not a silent flag.

## Hardware notes

- Prefer dedicated disk for WAL.
- RAM: SKLAD will use a buffer pool (`sklad.pool_bytes`).
- CPU: rustls + blake3 are the hot costs after IO.

## What "users keep the means of data hosting" is not

- It is not "every browser tab is a node."
- It is not a public DHT of private payroll.
- It is not an excuse to skip fsync.
