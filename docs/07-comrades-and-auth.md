# 07 -- Comrades and auth

There are no users. There are **comrades**, **komitets**, and **bilets**.

## Identities

| Kind | What it is |
| --- | --- |
| Node id | Ed25519 key for the host process. Speaks mesh. |
| Comrade id | Ed25519 key for a person or service. Speaks NashCQL. |
| Komitet | Named group of comrade ids. Can NAGRAD bilets. |

A comrade may hold keys on a laptop and never run a node.
A node operator is usually also a comrade, but the roles are separate
so a stolen laptop is not a stolen disk (and the reverse).

## Bootstrap

`oursqld init --data DIR` writes:

- `node.key` (mode 0600)
- founding komitet `FOUNDERS` with a god-bilet on `founder`
- intensity default 25
- `authz.json` next to the key (bilets, not passwords)

There is no password file. There is no `IDENTIFIED BY`.

## Session start

1. Client opens the node (line protocol, or OCHERED/1 frames).
2. Client sends `HELLO COMRADE mill`.
3. Node checks the comrade is known and not expired.
4. Node returns a dossier `DOS-...` and the current intensity.

Passwords may exist later as an **optional** wrap around the comrade
key (unlock the key file). They are never the network secret.

## Bilet (the capability)

This is a real type in `oursql-authz` (`Capability`). Field names are
NashCQL-shaped and that is what `POKAZ BILET` prints. Old JSON aliases
(`holder`, `verbs`, `not_after_epoch`) still load.

```
nagrad = {
  bilet,      // ticket id, BIL-000001
  comrade,    // who holds it
  deystv,     // OBTAN / INZRT / OPDAT / REMOV / MANUFAKTUR / CHEKA / ...
  predel,     // tabl name, or * for the whole kollektiv
  nachat,     // not before (unix). PUSTO = already live
  srok,       // not after (unix). PUSTO = no expiry. CHEKA always has one
  komitet,    // who stamped it (FOUNDERS)
  uslov       // { ration, max_rows, samokrit }
}
```

```
NAGRAD OBTAN NA COMRADE mill PREDEL bolts SROK 3600;
POKAZ BILET;
OTYAT OBTAN IZ COMRADE mill;
```

Authorization is **union** of live bilets for that comrade. Missing
deystv = deny. A bilet with `predel = bolts` cannot INZRT into `secrets`.

`NAGRAD` and `OTYAT` require an ADMIN / god bilet (the founders have one).

## CHEKA

`CHEKA` is a deystv, not a person:

- CONFISKAT / OSVOBOD
- POKAZ AUDIT
- set intensity (if so constrained)

CHEKA bilets must expire (<= 24h recommended, 7d hard max). Renewal is a
new NAGRAD. `SROK 0` is already dead -- useful in tests.

## Service comrades

Apps get a comrade of their own, issued by the plant komitet, scoped
with `PREDEL`, with a ration. This is how you avoid putting a human
key in a container.

## Signing modes

1. **Node-signed**: the node signs while the session comrade has no key.
2. **Comrade-signed**: after `HELLO ... KEY ... PODPIS`, every mutation
   needs that comrade's `PODPIS` (clause, `USTANOV podpis`, or OCHERED
   `0x09`). The WAL stores the signer so reopen verifies that key.

Unsigned mutations are refused (`2110 UNSIGNED_MUTATION`).

## What we will not add

- `COMRADE ''@'%'` anonymous.
- Shared password tables as the primary factor.
- OAuth inside the engine. Put that in the app.


## Komitet

`NAGRAD` is not a free gift. The issuer must sit on the komitet
(founders start on it). `NAGRAD KOMITET NA COMRADE mill` adds a voter.
`OTYAT KOMITET IZ COMRADE mill` removes one; the last seat cannot leave.
A mill with ADMIN but no seat gets `2111 NOT_KOMITET`.

## Uslov on a bilet

`NAGRAD OBTAN NA COMRADE mill RATION 20 MAXROWS 5 SAMOKRIT`

- `RATION` -- that comrade's statement budget (session)
- `MAXROWS` -- OBTAN is truncated (partial notice)
- `SAMOKRIT` -- mutations need a confession
