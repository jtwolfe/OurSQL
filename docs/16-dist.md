# 16 — Distro brigades

Each compiled pair (`oursql` + `oursqld`) is a hosting brigade.
Built on 2026-08-18 from this tree with `blake3` in pure Rust so we
do not need a C cross compiler.

## Linked binaries (this host could emit)

| Target | Kind |
| --- | --- |
| x86_64-unknown-linux-gnu | native ELF |
| x86_64-unknown-linux-musl | static musl |
| i686-unknown-linux-musl | 32-bit musl |
| aarch64-unknown-linux-musl | ARM64 musl |
| armv7-unknown-linux-musleabihf | ARMv7 musl |
| powerpc64le-unknown-linux-musl | ppc64le musl |
| wasm32-wasip1 | WASI wasm |

## Compiled but not linked here (no sysroot)

Windows GNU / gnullvm, Apple Darwin, FreeBSD, s390x, riscv64, aarch64-gnu.
`cargo check --target` succeeds. Link on a box that has that libc.

Rebuild everything:

```
bash scripts/build-all-arches.sh
```
