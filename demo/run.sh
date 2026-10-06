#!/usr/bin/env bash
# Starts the OpenHub QR demo: three Cloudflare quick tunnels and the Python,
# TypeScript and Ruby services, each using its own openhub-bo library.
#
#   demo/run.sh            # reads CLIENT_ID / CLIENT_SECRET from ../.env
#
# Prints the three public URLs and the access password; Ctrl+C stops everything.
# Quick tunnels need no Cloudflare account changes and live only while running.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUN="$ROOT/demo/.run"
mkdir -p "$RUN"
set -a; . "$ROOT/.env"; set +a
export DEMO_PASSWORD="${DEMO_PASSWORD:-$(python3 -c 'import secrets;print(secrets.token_urlsafe(12))')}"
umask 077; echo "$DEMO_PASSWORD" > "$RUN/password"

declare -A PORT=([python]=8101 [typescript]=8102 [ruby]=8103)
declare -A NAME=([python]=Python [typescript]=TypeScript [ruby]=Ruby)
pids=()
cleanup() { kill "${pids[@]}" 2>/dev/null || true; }
trap cleanup EXIT INT TERM

declare -A URL
for lang in python typescript ruby; do
  cloudflared tunnel --no-autoupdate --url "http://localhost:${PORT[$lang]}" > "$RUN/tunnel-$lang.log" 2>&1 &
  pids+=($!)
done
for lang in python typescript ruby; do
  for _ in $(seq 1 30); do
    URL[$lang]=$(grep -oE 'https://[a-z0-9-]+\.trycloudflare\.com' "$RUN/tunnel-$lang.log" | head -1 || true)
    [ -n "${URL[$lang]}" ] && break; sleep 1
  done
  [ -n "${URL[$lang]}" ] || { echo "tunnel for $lang did not start, see $RUN/tunnel-$lang.log"; exit 1; }
done
export DEMO_LINKS="Python=${URL[python]},TypeScript=${URL[typescript]},Ruby=${URL[ruby]}"

(cd "$ROOT/bindings/python" && PORT=8101 PUBLIC_URL="${URL[python]}" \
  exec uv run --no-sync python "$ROOT/demo/python/server.py") > "$RUN/python.log" 2>&1 &
pids+=($!)
(cd "$ROOT/demo/typescript" && PORT=8102 PUBLIC_URL="${URL[typescript]}" exec node server.ts) \
  > "$RUN/typescript.log" 2>&1 &
pids+=($!)
(cd "$ROOT/demo/ruby" && PORT=8103 PUBLIC_URL="${URL[ruby]}" exec ruby \
  -I"$ROOT/bindings/ruby/openhub-bo-core/lib" -I"$ROOT/bindings/ruby/openhub-bo-qr/lib" server.rb) \
  > "$RUN/ruby.log" 2>&1 &
pids+=($!)

for lang in python typescript ruby; do
  for _ in $(seq 1 30); do curl -sf "http://localhost:${PORT[$lang]}/health" >/dev/null && break; sleep 1; done
done
echo
echo "OpenHub QR demo (usuario: demo, clave en $RUN/password)"
for lang in python typescript ruby; do printf '  %-10s %s\n' "${NAME[$lang]}" "${URL[$lang]}"; done
echo "Logs en $RUN/. Ctrl+C para detener."
wait
