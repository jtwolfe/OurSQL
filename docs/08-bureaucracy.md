# 08 — Bureaucracy (the 25% layer)

This is the oppressive part. It is a crate. It has tests. It does not
touch pages.

Config key: `bureau.intensity` in `0..=100`. Default **25**.

## Design rule

Every bureau behavior has:

- a name
- an intensity threshold
- a deterministic hook (seeded RNG from `(dossier, stmt_hash, epoch)` so
  retries are stable for a short window)
- an error / notice code
- a usefulness note (what still works)

## Behaviors

### B1. Sharing delay / partial results

- Threshold: 20
- At 25: ~8% of OBTAN plans mark a random subset of qualifying page ids
  as `LOANED` for 50-400ms. First response is short, with
  `1902 COLLECTIVE_PARTIAL` and a `retry_after_ms`.
- Retry with the same `plan_id` returns the missing rows. After the loan
  expires, a fresh OBTAN is complete.
- Usefulness: apps retry once. Data is not wrong, only late.

### B2. Soft collective review

- Threshold: 15 for DDL, 25 for "large" mutations (row estimate >= 1000
  or statement text >= 8KiB)
- At 25: sleep 40-180ms, then proceed. No vote.
- At 60: another session or a komitet member must `NAGRAD APPROVAL`
  within `bureau.approval_ms` or the mutation fails `1904 NO_APPROVAL`.
- Usefulness: 25 is just latency.

### B3. Capitalist excess / gulag

- Threshold: 10
- Token bucket per comrade: `ration_qps` (default 40) and
  `ration_burst` (default 80).
- On exhaust: `1905 GULAG` with `retry_after_ms` (1-15s). Session stays
  open. No data loss.
- Message (ASCII): `Too capitalist. Temporary gulag. Retry later.`
- Usefulness: this is a rate limit with theater.

### B4. Accuse

```
ACCUSE COMRADE 'mill'@'plant-3' OF SPY SAMOKRIT 'odd ration pattern';
```

- Threshold: 25 to accept the verb
- At 25: write audit + 30s priority demotion (the accused yields in the
  scheduler). No ban. Accuser is rate-limited (3/day).
- At 60: accused OBTAN may be delayed. CHEKA is notified.
- At 100: accused session killed.
- Usefulness: 25 is social noise plus a mild scheduler hint.

### B5. Committee or arbitrary comrade approval

- Threshold: 60 for automatic requirement, 25 as an **opt-in** hint
  (`USTANOV require_approval = DA` on a tabl).
- A waiting mutation appears in `POKAZ OCHERED`. Any comrade with
  `APPROVE` on that scope can `NAGRAD APPROVAL DOS-...`.
- Usefulness: default tabls at 25 do not wait on humans.

### B6. KGB / CHEKA confiscation

```
CONFISKAT TABL payroll SAMOKRIT 'audit hold';
OSVOBOD TABL payroll;
```

- Threshold: 25 (verb exists), requires CHEKA cap
- Effect: OBTAN/INZRT/OPDAT/REMOV on the target return `1906 CONFISKAT`
  for non-CHEKA sessions. Pages stay. WAL stays.
- Holds expire (`bureau.confiskat_ttl`, default 24h) unless renewed.
- Usefulness: a real freeze tool with a joke name.

### B7. Samokrit

```
INZRT V parts ZNACH (...) SAMOKRIT 'serves depot 2';
```

- Threshold: 25 encourages, 50 requires on mutations
- At 25: optional. If present, stored in audit. Tiny scheduler bump.
- Usefulness: optional comment field.

### B8. Keyword snobbery

- Threshold: 40 to reject decadent SQL
- At 25: accept SQL, rewrite, emit `1901 BOURGEOIS_KEYWORDS`.
- Usefulness: existing SQL clients work.

## What bureaucracy must never do

- Skip WAL
- Drop a certified row
- Invent a second privilege system
- Block intensity 0 writes
- Use wall-clock sleep inside storage locks

## Knobs (`node.toml`)

```toml
[bureau]
intensity = 25
ration_qps = 40
ration_burst = 80
partial_pct = 8
review_delay_ms = { min = 40, max = 180 }
confiskat_ttl = "24h"
accuse_per_day = 3
```

## Testing contract

`crates/oursql-bureau` tests must include:

- intensity 0: zero 19xx errors on a normal INZRT/OBTAN
- intensity 25: gulag fires when bucket empty; partial is retryable
- confiscation does not delete pages (storage mock asserts page still there)
- accusation at 25 does not close the accused session
