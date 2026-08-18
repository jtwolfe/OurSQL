# 10 — Wire protocol

Name: **OCHERED/1** (the queue).

Two surfaces, one IR:

1. **Text** — NashCQL (and decadent SQL at intensity <= 40)
2. **Binary** — length-prefixed envelopes for drivers

## Transport

TLS 1.3, mTLS. ALPN: `oursql/1`.

## Binary framing

```
u32be length     # bytes after this field, max 16MiB
u8   flags       # 0x01 compressed (reserved, off)
u8   type        # see table
u16  reserved
[u8; length-4] payload
```

### Message types

| Type | Name | Direction |
| --- | --- | --- |
| 0x01 | HELLO | C->S |
| 0x02 | WELCOME | S->C |
| 0x03 | STMT | C->S |
| 0x04 | BIND | C->S |
| 0x05 | ROWS | S->C |
| 0x06 | DONE | S->C |
| 0x07 | NOTICE | S->C |
| 0x08 | ERROR | S->C |
| 0x09 | ACCUSE | C->S |
| 0x0A | APPROVAL | C->S |
| 0x0B | PING / PONG | both |

HELLO payload: comrade pubkey, client nonce, client name, protocol minor.

WELCOME: session dossier, intensity, node id, view epoch, feature bits.

STMT: utf-8 NashCQL or a prepared handle.

ERROR: `u16 code`, `u16 retry_after_ms`, utf-8 message (ASCII).

## Text mode

A line-oriented fallback for humans (`oursql-cli`). Prompt:

```
nashcql [sklad] DOS-014882>
```

Statements end with `;`. Multi-line allowed.

## Versioning

Breaking changes increment `oursql/N`. Old nodes refuse newer ALPN.
Newer clients may speak older ALPN if advertised.

## Backpressure

If the session worker queue is full: `2108 NODE_BUSY` rather than
unbounded memory. This is not a gulag (gulag is per-comrade ration).
