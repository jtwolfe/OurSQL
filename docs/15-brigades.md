# 15 — Brigades (code segmentation)

The means of production are not a ministry. Each crate is a **brigade**
with one job. Cyclic deps are banned. `oursql-bureau` MUST NOT depend
on `oursql-storage`.

```
                    +-----------+
                    |   CORE    |  types, errors, intensity
                    +-----+-----+
                          |
          +---------------+---------------+
          |               |               |
     +----+----+    +-----+-----+   +-----+-----+
     | CRYPTO  |    |  NASHCQL  |   |   WIRE    |
     +----+----+    +-----+-----+   +-----+-----+
          |               |
     +----+----+    +-----+-----+
     |  SKLAD  |    |  BUREAU   |
     | (store) |    | (policy)  |
     +----+----+    +-----+-----+
          |               |
          +-------+-------+
                  |
            +-----+-----+     +--------+
            |   AUTHZ   |     |  MESH  |
            +-----+-----+     +---+----+
                  |               |
                  +-------+-------+
                          |
                    +-----+-----+
                    |   SOYUZ   |  engine session
                    +-----+-----+
                       /     \
                +-----+       +-----+
                | CLI |       | NODE|
                +-----+       +-----+
                    \
                  DRIVER
```

| Brigade | Crate | Owns | Forbidden |
| --- | --- | --- | --- |
| CORE | oursql-core | Intensity, Error, Value | IO, parse |
| CRYPTO | oursql-crypto | BLAKE3, Ed25519, CRC | files |
| SKLAD | oursql-storage | WAL, tabls, recover | NashCQL, gulag |
| NASHCQL | oursql-nashcql | lex, parse, IR | execute |
| BUREAU | oursql-bureau | gulag, accuse, delay | pages |
| AUTHZ | oursql-authz | caps, comrades | WAL |
| MESH | oursql-consensus | certify digests | parse |
| WIRE | oursql-wire | OCHERED/1 frames | policy |
| SOYUZ | oursql-engine | session execute | (assembles) |
| DRIVER | oursql-driver | TCP client | storage |
| CLI | oursql-cli | `oursql` bin | server |
| NODE | oursql-node | `oursqld` bin | planner guts |

Tests live **in the brigade they judge**. Cross-brigade stories live in
`oursql-engine/tests`.
