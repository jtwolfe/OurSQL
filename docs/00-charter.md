# 00 -- Charter

## One sentence

OurSQL is a Rust database that is hosted by its users, certified by a mesh,
queried in NashCQL, and governed by an explicit bureaucracy that is annoying
on purpose.

## The political claim (engineering, not party)

Most databases assume:

1. One operator owns the machine.
2. Clients are guests.
3. Privilege is a private grant from an administrator.
4. Performance is the highest virtue.

OurSQL assumes the inverse, then compromises just enough to remain a database:

1. **Many operators** own machines. The store is a collective of hosts.
2. Clients are **comrades with dossiers**, not anonymous sockets.
3. Privilege is a **capability** issued by a komitet, time-boxed, and auditable.
4. Correctness and **host sovereignty** outrank raw single-node throughput.
5. A thin, tunable **bureaucracy** sits on the hot path so the social model
   cannot be "turned off in production and forgotten."

This is a technical project with a satirical surface. The satire is specified
so it cannot rot into undefined behavior.

## Design axioms

1. **Memory safety is not optional.** Default code paths contain no `unsafe`.
   Any `unsafe` block is a numbered exception in `docs/09-security.md` with a
   safety comment and a test.
2. **Users keep the means of data hosting.** A node that is off the mesh still
   holds its pages. Leaving the mesh does not delete local durable state.
3. **No silent root.** There is no `COMRADE root@%` with eternal `SUPER`.
   Bootstrap produces a **founding komitet** of N capability holders.
4. **Writes are signed.** Every mutating statement carries a comrade signature
   over a canonical digest. Unsigned mutations are refused.
5. **Bureaucracy is a module, not a meme in comments.** Intensity `0..=100`
   is a first-class config. Default is **25**. Tests pin behavior at 0, 25, 60,
   and 100.
6. **ASCII only in the language and the docs.** US-keyboard. No combining
   marks, no lookalike letters.
7. **Complicated is allowed. Useless is not.** At intensity 25 a competent
   application with retry logic must be able to store and retrieve rows.

## Intensity doctrine

| Intensity | Intent |
| --- | --- |
| 0 | Straight engine. Bureaucracy emits notices only. |
| 25 | Default. Mild delay, rare partial results, gulag on abuse, optional notes. |
| 60 | Approvals on non-trivial writes. Accusations have teeth. |
| 100 | Party congress on DDL. Confiscation common. Demo-only. |

The storage engine, WAL, consensus, and crypto **do not change** with
intensity. Only the policy overlay does.

## What "oppressive and complicated" means in practice

- Nouns have **dossier numbers** (`DOS-014882`).
- Mutations may require a **samokrit** string (self-criticism / justification).
- Sessions that hammer the node are sent to a **temporary gulag** (rate limit).
- Comrades may **ACCUSE** other comrades. At 25 this is an audit event plus a
  short priority demotion.
- A **CHEKA** capability may **CONFISKAT** a tabl (quarantine, not delete).
- Keywords look like English after a bad night (`PERESTROJ`, `INZRT`).

None of these delete committed user data at intensity 25.

## Non-negotiable usefulness bar

A new operator must be able to:

1. Start a node on their own host.
2. Join or found a kollektiv (database).
3. MANUFAKTUR a tabl, INZRT a row, OBTAN it back.
4. Crash the process and recover the row from the WAL.
5. Add a second host and see the row certified onto it.

If any of those fail, the bureaucracy is irrelevant because the product is not
a database.
