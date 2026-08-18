# 02 — Threat model

We use a simple four-box model. Every feature must name which box it shrinks.

## Assets

- User rows and indexes
- WAL / redo
- Comrade identity keys
- Capability tokens
- Mesh membership list
- Audit / samokrit / accusation logs
- Confiscation holds (metadata, not a second copy of the data)

## Actors

| Actor | Trust | Notes |
| --- | --- | --- |
| Local operator | Semi-trusted | Owns disk and process. Can destroy their node. Cannot forge another comrade's signature. |
| Remote comrade node | Untrusted until authenticated | May be honest, lagging, or hostile. |
| Client application | Untrusted | Speaks NashCQL or the binary protocol. |
| Network | Hostile | Drop, replay, reorder, MITM. |
| CHEKA holder | Powerful, audited | Can CONFISKAT. Cannot silently drop audit. |
| Bureaucracy module | Trusted code, untrusted config | Intensity is operator-set. |

## Threats we accept responsibility for

### T1. Forged writes
Mitigation: Ed25519 signatures over a canonical mutation digest
`(kollektiv, tabl, schema_epoch, stmt_hash, ts, dossier)`. Replay window is
bounded. See [09](09-security.md).

### T2. Node impersonation
Mitigation: mutual TLS with pin-set or TOFU plus rotation ceremony. Node
identity != comrade identity. A stolen disk does not yield the comrade key
if the key is sealed (age / OS keyring / operator-held).

### T3. Split-brain certification
Mitigation: permissioned quorum. Writes that require `ZAVERSHIT SOYUZ` abort
if the view is below quorum. Local-only commits stay local and are marked
`UNCERTIFIED`.

### T4. Privilege escalation via decadent SQL
Mitigation: one parser, two lexers (NashCQL + SQL). Both produce the same IR.
Authorization runs on IR, never on text.

### T5. Bureaucracy as an oracle
A client must not learn other comrades' private rows by watching gulag
timings or accusation side channels. Rate-limit replies are constant-size.
Accusation effects at 25 do not reveal row contents.

### T6. Confiscation abuse
CONFISKAT is a hold, not a delete. It is itself a signed audit event.
Release (`OSVOBOD`) is a second signed event. Holds expire unless renewed.

### T7. Compromised intensity
If an attacker sets intensity to 0 to skip approvals, that change is a
signed config event and is visible in `POKAZ USTANOV`. It cannot be hidden
from the founding komitet.

## Threats we explicitly do not solve

- A hostile operator shreds their own disk.
- A majority of founding keys collude.
- Side-channel leakage via CPU cache on a shared box (document, do not claim).
- Legal compulsion against an operator. Host sovereignty includes the right
  to be raided; encryption-at-rest is the mitigation, not a miracle.

## Trust boundaries

```
[app] --tls--> [oursql-cli / driver]
                    |
                    v
              [oursql-node]
               /    |     \
        bureau   nashcql   consensus
               \    |     /
              [storage + wal]
                    |
                   disk
```

Bureau never writes pages. Consensus never parses SQL. Storage never
interprets capabilities beyond "this page key decrypts."
