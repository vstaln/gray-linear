#!/bin/sh
here="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
sys="$(uname -s)"
case "$sys" in
MINGW* | MSYS* | CYGWIN*) rel="x86_64-pc-windows-msvc/linear.exe" ;;
Darwin*)
    case "$(uname -m)" in
    arm64) rel="aarch64-apple-darwin/linear" ;;
    *) rel="x86_64-apple-darwin/linear" ;;
    esac ;;
*)
    case "$(uname -m)" in
    aarch64 | arm64) rel="aarch64-unknown-linux-musl/linear" ;;
    *) rel="x86_64-unknown-linux-musl/linear" ;;
    esac ;;
esac
if [ -x "$here/$rel" ]; then exec "$here/$rel" "$@"; fi
for cand in "$here/target/release/linear.exe" "$here/target/release/linear" "$here/target/debug/linear.exe" "$here/target/debug/linear"; do
    if [ -x "$cand" ]; then exec "$cand" "$@"; fi
done
echo "linear binary not found in $here" >&2
exit 1
