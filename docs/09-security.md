# 09 — Security

## Principles

1. Deny by default.
2. Sign what you mutate.
3. Encrypt what you store.
4. Authenticate what you speak to.
5. Audit what you permit.
6. Expire what you empower.
7. No `unsafe` on the default path.

## Crypto suite (phase 1)

| Use | Algorithm |
| --- | --- |
| Comrade / node identity | Ed25519 |
| Mutation digest | BLAKE3 |
| Page encryption | XChaCha20-Poly1305 |
| Wrapping kollektiv keys | X25519 + XChaCha20-Poly1305 |
| TLS | rustls, TLS 1.3 only, mTLS |

We do not implement these ourselves. We use reviewed crates
(`ed25519-dalek`, `blake3`, `chacha20poly1305`, `rustls`).

## Unsafe exceptions

Workspace `#![deny(unsafe_code)]`.

Allowed later, each with a file comment and a test:

| ID | Crate | Reason |
| --- | --- | --- |
| U1 | oursql-storage | optional SIMD checksum |
| U2 | oursql-crypto | none expected; document if a shim appears |

## Session hardening

- Max frame 16 MiB.
- Max in-flight statements per session: 2.
- Idle timeout required in the production profile.
- Bind `0.0.0.0` only if `listen.public = true` AND a pin-set is configured.
  Dev profile may bind localhost.

## Injection

One IR. Text is not concatenated into storage calls. Parameters are
bound (`$1`, `$2` or `:name`). String-building APIs are not provided.

## Audit

Every 19xx and every mutation writes an audit record:

```
(ts, dossier, comrade, verb, scope, digest, intensity, note)
```

Audit tabl `_meta.audit` is append-only. REMOV is refused. OCHISTKA
requires CHEKA + two founders.

## Key handling

- `node.key` mode 0600, refuse start otherwise.
- Comrade keys never written to WAL in raw form.
- Memory: keys live in `secrecy` / zeroizing types.
- Rotation: `PERESTROJ COMRADE ... ROTATE KEY` with overlap window.

## Supply chain

- `cargo deny` in CI once CI exists (licenses, advisories, bans).
- Lockfile committed.
- No `git` dependencies on default branch.

## Disclosure

See [SECURITY.md](../SECURITY.md). Report privately. We do not pay in
coins we refused to mint.
