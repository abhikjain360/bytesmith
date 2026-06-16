#!/usr/bin/env bash
# Deterministic bytesmith-vs-simple-dns DNS comparison (instruction + alloc counts).
#
# The two head-to-head programs live as examples of bytesmith-bench:
#   comparison/dns_bytesmith.rs   -> target/release/examples/dns_bytesmith
#   comparison/dns_simpledns.rs  -> target/release/examples/dns_simpledns
#
# Build + profile (Linux, valgrind):
#   CARGO_PROFILE_RELEASE_DEBUG=2 RUSTFLAGS="-C force-frame-pointers=yes" \
#     cargo build -p bytesmith-bench --examples --release
#   valgrind --tool=callgrind --cache-sim=no --branch-sim=no \
#     --callgrind-out-file=/tmp/cg_bp.out ./target/release/examples/dns_bytesmith 1000
#   valgrind --tool=callgrind --cache-sim=no --branch-sim=no \
#     --callgrind-out-file=/tmp/cg_sd.out ./target/release/examples/dns_simpledns 1000
#   bash comparison/analyze_callgrind.sh
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

bp=/tmp/cg_bp.out
sd=/tmp/cg_sd.out

echo "===== TOTAL Ir (N=1000 iterations) ====="
bpi=$(grep -m1 '^summary:' "$bp" | awk '{print $2}')
sdi=$(grep -m1 '^summary:' "$sd" | awk '{print $2}')
echo "bytesmith  : $bpi"
echo "simple-dns: $sdi"
gawk -v a="$bpi" -v b="$sdi" 'BEGIN{printf "ratio     : %.1fx more instructions\n", a/b}'

echo
echo "===== per-work() call counts (bytesmith, total/1000) ====="
gawk '
/^fn=\(/  { if (match($0,/^fn=\(([0-9]+)\) (.+)/,m))  nm[m[1]]=m[2] }
/^cfn=\(/ { if (match($0,/^cfn=\(([0-9]+)\)( (.+))?/,m)) { cur=m[1]; if (m[3]!="") nm[m[1]]=m[3] } }
/^calls=/ { c=$0; sub(/^calls=/,"",c); split(c,a," "); cnt[cur]+=a[1] }
END {
  for (k in cnt) {
    n=nm[k]
    if (n ~ /dns_name/ || n ~ /from_utf8/ || n ~ /__rust_alloc/ || n ~ /__rust_dealloc/ || n ~ /realloc/ || n ~ /join/ || n ~ /6Vec.*push/)
      printf "%9.1f  %s\n", cnt[k]/1000.0, n
  }
}' "$bp" | sort -rn | head -25

echo
echo "===== bytesmith top self-Ir functions ====="
callgrind_annotate --threshold=95 --auto=no "$bp" 2>/dev/null \
  | awk '/PROGRAM TOTALS/{p=1} p' | grep -E '^[ ]*[0-9]' | head -22

echo
echo "===== simple-dns top self-Ir functions ====="
callgrind_annotate --threshold=95 --auto=no "$sd" 2>/dev/null \
  | awk '/PROGRAM TOTALS/{p=1} p' | grep -E '^[ ]*[0-9]' | head -12
