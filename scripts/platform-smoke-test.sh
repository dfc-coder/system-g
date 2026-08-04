#!/usr/bin/env bash
set -euo pipefail

case "${GROWNTROL_COMPOSE_PROVIDER:-auto}" in
  podman)
    compose=(podman compose --profile demo)
    ;;
  podman-compose)
    compose=(podman-compose --profile demo)
    ;;
  docker)
    compose=(docker compose --profile demo)
    ;;
  auto)
    if command -v podman-compose >/dev/null 2>&1; then
      compose=(podman-compose --profile demo)
    elif podman compose version >/dev/null 2>&1; then
      compose=(podman compose --profile demo)
    elif docker compose version >/dev/null 2>&1; then
      compose=(docker compose --profile demo)
    else
      printf 'No Compose provider found. Install Podman Compose or Docker Compose.\n' >&2
      exit 1
    fi
    ;;
  *)
    printf 'Unsupported GROWNTROL_COMPOSE_PROVIDER: %s\n' "$GROWNTROL_COMPOSE_PROVIDER" >&2
    exit 1
    ;;
esac

cleanup() {
  "${compose[@]}" logs --no-color || true
  "${compose[@]}" down --volumes --remove-orphans || true
}
trap cleanup EXIT

"${compose[@]}" up --build --detach

for _ in $(seq 1 90); do
  if curl --fail --silent http://127.0.0.1:8080/health >/dev/null; then
    break
  fi
  sleep 1
done
curl --fail --silent http://127.0.0.1:8080/health >/dev/null

for _ in $(seq 1 90); do
  devices="$(curl --fail --silent http://127.0.0.1:8080/api/devices || true)"
  if python3 -c '
import json, sys
records = json.loads(sys.argv[1])
raise SystemExit(0 if any(item.get("device_id") == "demo-grow-1" for item in records) else 1)
' "$devices"; then
    break
  fi
  sleep 1
done

curl --fail --silent http://127.0.0.1:8080/api/devices/demo-grow-1 >/dev/null

command_response="$(
  curl --fail --silent \
    --request POST \
    --header 'Content-Type: application/json' \
    --data '{"type":"set_override","target":"fans","mode":"on","duration_seconds":1800}' \
    http://127.0.0.1:8080/api/devices/demo-grow-1/commands
)"

command_id="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["command_id"])' "$command_response")"

for _ in $(seq 1 30); do
  device="$(curl --fail --silent http://127.0.0.1:8080/api/devices/demo-grow-1)"
  if python3 -c '
import json, sys
record = json.loads(sys.argv[1])
command_id = sys.argv[2]
ack = record.get("latest_acknowledgement") or {}
raise SystemExit(0 if ack.get("command_id") == command_id and ack.get("accepted") is True else 1)
' "$device" "$command_id"; then
    exit 0
  fi
  sleep 1
done

printf 'Simulator did not acknowledge command %s\n' "$command_id" >&2
exit 1
