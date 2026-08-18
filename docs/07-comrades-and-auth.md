# 07 — Comrades and auth

There are no users. There are **comrades**, **komitets**, and **capabilities**.

## Identities

| Kind | What it is |
| --- | --- |
| Node id | Ed25519 key for the host process. Speaks mesh. |
| Comrade id | Ed25519 key for a person or service. Speaks NashCQL. |
| Komitet | Named group of comrade ids. Can NAGRAD capabilities. |

A comrade may hold keys on a laptop and never run a node.
A node operator is usually also a comrade, but the roles are separate
so a stolen laptop is not a stolen disk (and the reverse).

## Bootstrap

`oursqld init` writes:

- `node.key`
- founding komitet `FOUNDERS` with N (default 3, allowed 1) comrade
  public keys passed on the command line
- intensity default 25
- a self-kollektiv `_meta` for authz tables

There is no password file. There is no `IDENTIFIED BY`.

## Session start

1. Client opens mTLS with the node (node cert).
2. Client sends `HELLO` + comrade public key + signed nonce.
3. Node checks the comrade is not in gulag and not `UNRELIABLE`.
4. Node returns a session dossier `DOS-...` and the current intensity.

Passwords may exist later as an **optional** wrap around the comrade
key (unlock the key file). They are never the network secret.

## Capabilities

A capability is a signed tuple:

```
cap = {
  id,
  holder,          // comrade
  verbs,           // OBTAN, INZRT, ...
  scope,           // kollektiv / tabl / column
  not_before,
  not_after,
  issued_by,       // komitet
  constraints      // ration, max_rows, require_samokrit
}
```

Authorization is **intersection** of all active caps. Missing verb = deny.

`NAGRAD` and `OTYAT` are themselves mutations on `_meta` and require a
komitet cap.

## CHEKA

`CHEKA` is a verb set, not a person:

- CONFISKAT / OSVOBOD
- POKAZ AUDIT
- force-disconnect a session
- set intensity (if so constrained)

CHEKA caps must expire (<= 24h recommended, 7d hard max). Renewal is a
new NAGRAD.

## Service comrades

Apps get a comrade key of their own, issued by the plant komitet, scoped
to one kollektiv, with a ration. This is how you avoid putting a human
key in a container.

## Signing modes

1. **Node-signed** (default at 25): the node signs mutations on behalf of
   an authenticated session. Faster. The session handshake is the trust.
2. **Comrade-signed**: the client signs each mutation. Required for
   `ZAVERSHIT CHEKA` and for intensity >= 60 on DDL.

## What we will not add

- `COMRADE ''@'%'` anonymous.
- Shared password tables as the primary factor.
- OAuth inside the engine. Put that in the app.
