#!/usr/bin/env bash
# Compile oursql + oursqld for every target this host can reasonably emit.
# Segmentation: each arch is a brigade. Failures are logged, not fatal.
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
OUT="${OURL_DIST:-$ROOT/dist}"
mkdir -p "$OUT"
HOST="$(rustc -vV | awk '/host:/{print $2}')"
echo "host=$HOST"

TARGETS=(
  x86_64-unknown-linux-gnu
  x86_64-unknown-linux-musl
  aarch64-unknown-linux-gnu
  aarch64-unknown-linux-musl
  i686-unknown-linux-gnu
  i686-unknown-linux-musl
  armv7-unknown-linux-musleabihf
  armv7-unknown-linux-gnueabihf
  riscv64gc-unknown-linux-gnu
  riscv64gc-unknown-linux-musl
  powerpc64le-unknown-linux-gnu
  s390x-unknown-linux-gnu
  loongarch64-unknown-linux-gnu
  loongarch64-unknown-linux-musl
  wasm32-wasip1
  wasm32-unknown-unknown
  x86_64-pc-windows-gnu
  x86_64-pc-windows-gnullvm
  i686-pc-windows-gnu
  x86_64-unknown-freebsd
  x86_64-unknown-netbsd
  x86_64-apple-darwin
  aarch64-apple-darwin
  aarch64-unknown-linux-ohos
)

ok=0
fail=0
skip=0
summary="$OUT/SUMMARY.txt"
: > "$summary"

add_target() {
  local t="$1"
  rustup target add "$t" >/tmp/oursql-target-add.log 2>&1 || return 1
  return 0
}

build_one() {
  local t="$1"
  local dest="$OUT/$t"
  mkdir -p "$dest"
  local extra=()
  case "$t" in
    *musl*|wasm32-*|*-windows-gnullvm)
      extra+=(-C linker=rust-lld)
      ;;
  esac
  case "$t" in
    riscv64gc-unknown-linux-musl|loongarch64-unknown-linux-musl)
      extra+=(-C target-feature=+crt-static)
      ;;
  esac
  # library-only targets
  if [ "$t" = "wasm32-unknown-unknown" ]; then
    if RUSTFLAGS="${extra[*]}" cargo build --release --target "$t" \
        -p oursql-engine -p oursql-core -p oursql-storage \
        >"$dest/build.log" 2>&1; then
      echo "OK   $t (libs)" | tee -a "$summary"
      return 0
    else
      echo "FAIL $t (libs)" | tee -a "$summary"
      return 1
    fi
  fi
  if RUSTFLAGS="${extra[*]}" cargo build --release --target "$t" \
      -p oursql-cli -p oursql-node >"$dest/build.log" 2>&1; then
    mkdir -p "$dest/bin"
    find "target/$t/release" -maxdepth 1 -type f \( -name oursql -o -name oursqld -o -name 'oursql.exe' -o -name 'oursqld.exe' -o -name 'oursql.wasm' -o -name 'oursqld.wasm' \) \
      -exec cp {} "$dest/bin/" \;
    echo "OK   $t" | tee -a "$summary"
    return 0
  fi
  # fallback: rustc check (no link)
  if cargo check --target "$t" -p oursql-engine >"$dest/check.log" 2>&1; then
    echo "CHECK $t (no linker)" | tee -a "$summary"
    return 0
  fi
  echo "FAIL $t" | tee -a "$summary"
  return 1
}

# Always build host first.
if cargo build --release -p oursql-cli -p oursql-node; then
  mkdir -p "$OUT/$HOST/bin"
  cp -f target/release/oursql target/release/oursqld "$OUT/$HOST/bin/" 2>/dev/null || true
  echo "OK   $HOST (native)" | tee -a "$summary"
  ok=$((ok+1))
else
  echo "FAIL $HOST native" | tee -a "$summary"
  fail=$((fail+1))
fi

for t in "${TARGETS[@]}"; do
  if [ "$t" = "$HOST" ]; then
    continue
  fi
  if ! add_target "$t"; then
    echo "SKIP $t (no rust-std)" | tee -a "$summary"
    skip=$((skip+1))
    continue
  fi
  if build_one "$t"; then
    ok=$((ok+1))
  else
    fail=$((fail+1))
  fi
done

echo "" | tee -a "$summary"
echo "brigades ok=$ok fail=$fail skip=$skip" | tee -a "$summary"
ls -la "$OUT"/*/bin 2>/dev/null | tee -a "$summary" || true
exit 0
