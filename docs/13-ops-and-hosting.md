# 13 — Ops and hosting

## The point

You run the node. You keep the disk. The mesh is a treaty, not a landlord.

## Init

```
oursqld init --data /var/lib/oursql --intensity 25
```

Writes `node.key` (0600), `authz.json` (FOUNDERS god-bilet), empty WAL.

## Run a lonely plant

```
oursqld run --data /var/lib/oursql --listen 127.0.0.1:3307 --intensity 25
```

Talk to it:

```
oursql --data /var/lib/oursql --intensity 0 -f examples/hello-kollektiv.nql
# or
printf 'POKAZ TABL;\n' | nc 127.0.0.1 3307
```

## Run a kollektiv (four plants)

Each plant has its own disk. `--peer` is the **mesh** address, not the
NashCQL listen address.

```
oursqld run --data /a --name alpha --listen 127.0.0.1:3307 --mesh 127.0.0.1:3401 \
  --peer 127.0.0.1:3402 --peer 127.0.0.1:3403 --peer 127.0.0.1:3404 --intensity 0
oursqld run --data /b --name beta  --listen 127.0.0.1:3308 --mesh 127.0.0.1:3402 \
  --peer 127.0.0.1:3401 --intensity 0
# gamma, delta the same
```

Then on alpha:

```
NACHAT;
MANUFAKTUR TABL bolts (id NARODKEY, qty CELIY);
INZRT V bolts (id, qty) ZNACH ('b1', 40);
ZAVERSHIT SOYUZ;
```

Beta can `OBTAN * IZ bolts`. A fifth plant with `--peer 127.0.0.1:3401`
sends `NEED` at boot and receives a `SNAPSHOT`.

HTTP clerk (optional): `--admin 127.0.0.1:3309` then
`GET /health`, `GET /pokaz`, `POST /nql`.

## Backup

Cold: stop node, copy the data dir (`node.key`, `wal.log`,
`kollektiv/pages/checkpoint.pg`, `authz.json`). Tar + checksum is enough.

## Restore

Copy data dir, start. If the plant is behind the mesh, `--peer` an
up-to-date node; NEED/SNAPSHOT fills it.

## Intensity change

```
USTANOV intensity = 25;
```

Local to that plant. Intensity is not a mesh vote.

## What "users keep the means of data hosting" is not

- It is not "every browser tab is a node."
- It is not a public DHT of private payroll.
- It is not an excuse to skip fsync.
