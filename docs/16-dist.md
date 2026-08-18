# 16 -- Distro brigades

Each compiled pair (`oursql` + `oursqld`) is a hosting brigade.
`v0.2.0` binaries were linked on 2026-08-18 from this tree with
`blake3` in pure Rust so we do not need a C cross compiler. Musl
targets use `rust-lld` (`-C linker=rust-lld`). RISC-V and LoongArch
also use `+crt-static`.

## Linked binaries (this host could emit)

| Target | Kind |
| --- | --- |
| x86_64-unknown-linux-gnu | native ELF |
| x86_64-unknown-linux-musl | static musl |
| i686-unknown-linux-musl | 32-bit musl |
| aarch64-unknown-linux-musl | ARM64 musl |
| armv7-unknown-linux-musleabihf | ARMv7 musl |
| powerpc64le-unknown-linux-musl | ppc64le musl |
| riscv64gc-unknown-linux-musl | RISC-V musl, crt-static |
| loongarch64-unknown-linux-musl | LoongArch musl, crt-static |
| wasm32-wasip1 | WASI wasm |

Get them from the [v0.2.0 release](https://github.com/jtwolfe/OurSQL/releases/tag/v0.2.0).
Each tarball has `oursql`, `oursqld` (or `.wasm`), and `README.txt`.
`SHA256SUMS` sits next to the tarballs.

## Compiled but not linked here (no sysroot)

Windows GNU / gnullvm, Apple Darwin, FreeBSD, s390x-gnu, aarch64-gnu.
`cargo check --target` succeeds. Link on a box that has that libc.

Rebuild everything:

```
bash scripts/build-all-arches.sh
```
