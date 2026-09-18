#!/usr/bin/env bash
# Gateflow dev entrypoint: mock upstream (:9099) + gateway (:8081) + web (:3000).
#
#   bash scripts/dev.sh [--no-watch] [--no-free-ports] [--no-install]
#
# Hot reload: `cargo watch` restarts Rust on crates/** change;
# Next.js HMR handles web. All children share one trap-cleaned process
# group with prefixed logs.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MOCK_PORT="${MOCK_PORT:-9099}"
GATEFLOW_PORT="${GATEFLOW_PORT:-8081}"
WEB_PORT="${WEB_PORT:-3000}"

export GATEFLOW_BIND="${GATEFLOW_BIND:-127.0.0.1:${GATEFLOW_PORT}}"
export DATABASE_URL="${DATABASE_URL:-sqlite://gateflow.db}"
export GATEFLOW_CORS_ORIGINS="${GATEFLOW_CORS_ORIGINS:-http://localhost:${WEB_PORT}}"
export NEXT_PUBLIC_API_URL="${NEXT_PUBLIC_API_URL:-http://localhost:${GATEFLOW_PORT}}"
export RUST_LOG="${RUST_LOG:-gateflow=debug,tower_http=info,axum=info}"

WATCH=true
FREE_PORTS=true
INSTALL=true
for arg in "$@"; do
  case "$arg" in
    --no-watch) WATCH=false ;;
    --no-free-ports) FREE_PORTS=false ;;
    --no-install) INSTALL=false ;;
    -h|--help)
      echo "usage: scripts/dev.sh [--no-watch] [--no-free-ports] [--no-install]"
      exit 0
      ;;
    *) echo "unknown flag: $arg (see --help)" >&2; exit 1 ;;
  esac
done

need() {
  command -v "$1" >/dev/null 2>&1 || { echo "missing required tool: $1" >&2; exit 1; }
}

need cargo
need npm
need node

if "$INSTALL"; then
  if ! command -v cargo-watch >/dev/null 2>&1; then
    echo "[dev] installing cargo-watch…"
    cargo install cargo-watch --locked
  fi
  if [ ! -d web/node_modules ]; then
    echo "[dev] installing web dependencies…"
    npm install --prefix web
  fi
elif ! command -v cargo-watch >/dev/null 2>&1 && "$WATCH"; then
  echo "cargo-watch missing (re-run without --no-install, or pass --no-watch)" >&2
  exit 1
fi

free_port() {
  if command -v fuser >/dev/null 2>&1; then
    fuser -k "$1/tcp" 2>/dev/null || true
  elif command -v lsof >/dev/null 2>&1; then
    lsof -ti tcp:"$1" | xargs -r kill 2>/dev/null || true
  else
    echo "[dev] cannot free port $1: need fuser or lsof" >&2
  fi
}

if "$FREE_PORTS"; then
  echo "[dev] freeing ports ${GATEFLOW_PORT} ${MOCK_PORT} ${WEB_PORT}…"
  free_port "$MOCK_PORT"
  free_port "$GATEFLOW_PORT"
  free_port "$WEB_PORT"
fi

PIDS=()
prefix() { sed -e "s/^/[$1] /"; }

cleanup() { kill "${PIDS[@]}" 2>/dev/null || true; }
trap cleanup SIGINT SIGTERM EXIT

MOCK_CMD=(cargo run -p gateflow-server --example mock_upstream)
API_CMD=(cargo run -p gateflow-server)
if "$WATCH"; then
  MOCK_CMD=(cargo watch -q -w crates/server/examples -x "run -p gateflow-server --example mock_upstream")
  API_CMD=(cargo watch -q -c -w crates -x "run -p gateflow-server")
fi

echo "[dev] mock     http://localhost:${MOCK_PORT} (${MOCK_CMD[*]})"
echo "[dev] gateway  http://${GATEFLOW_BIND} (DATABASE_URL=${DATABASE_URL})"
echo "[dev] web      http://localhost:${WEB_PORT} (NEXT_PUBLIC_API_URL=${NEXT_PUBLIC_API_URL})"

"${MOCK_CMD[@]}" 2>&1 | prefix mock &
PIDS+=($!)
"${API_CMD[@]}" 2>&1 | prefix api &
PIDS+=($!)
npm run dev --prefix web -- --port "$WEB_PORT" 2>&1 | prefix web &
PIDS+=($!)

wait
