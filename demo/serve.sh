#!/usr/bin/env bash
# Starts the three demo services using the tunnel URLs written by run.sh
# (demo/.run/urls.env). Run it alone to restart the services without changing
# the public URLs; Ctrl+C stops them.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUN="$ROOT/demo/.run"
set -a; . "$ROOT/.env"; . "$RUN/urls.env"; set +a
export DEMO_PASSWORD="${DEMO_PASSWORD:-$(cat "$RUN/password")}"
export DEMO_LINKS="Python=$URL_PYTHON,TypeScript=$URL_TYPESCRIPT,Ruby=$URL_RUBY"

pids=()
trap 'kill "${pids[@]}" 2>/dev/null || true' EXIT INT TERM
(cd "$ROOT/bindings/python" && PORT=8101 PUBLIC_URL="$URL_PYTHON" \
  exec uv run --no-sync python "$ROOT/demo/python/server.py") > "$RUN/python.log" 2>&1 &
pids+=($!)
(cd "$ROOT/demo/typescript" && PORT=8102 PUBLIC_URL="$URL_TYPESCRIPT" exec node server.ts) \
  > "$RUN/typescript.log" 2>&1 &
pids+=($!)
(cd "$ROOT/demo/ruby" && PORT=8103 PUBLIC_URL="$URL_RUBY" exec ruby \
  -I"$ROOT/bindings/ruby/openhub-bo-core/lib" -I"$ROOT/bindings/ruby/openhub-bo-qr/lib" server.rb) \
  > "$RUN/ruby.log" 2>&1 &
pids+=($!)
for port in 8101 8102 8103; do
  for _ in $(seq 1 30); do curl -sf "http://localhost:$port/health" >/dev/null && break; sleep 1; done
done
echo "services up (logs in $RUN/)"
wait
