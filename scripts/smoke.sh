#!/usr/bin/env bash
set -euo pipefail

API_BASE_URL="${API_BASE_URL:-https://api.linkdrop.ahara.io}"
API_BASE_URL="${API_BASE_URL%/}"

curl_json() {
  curl --fail --silent --show-error -H "Accept: application/json" "$@"
}

auth_curl() {
  curl_json -H "Authorization: Bearer ${LINKDROP_ACCESS_TOKEN}" "$@"
}

json_escape() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  printf '%s' "${value}"
}

check_health() {
  echo "Checking ${API_BASE_URL}/health"
  curl_json "${API_BASE_URL}/health" >/dev/null
}

check_authenticated_reads() {
  if [[ -z "${LINKDROP_ACCESS_TOKEN:-}" ]]; then
    echo "Skipping authenticated checks; LINKDROP_ACCESS_TOKEN is not set."
    return
  fi

  echo "Checking authenticated API routes"
  auth_curl "${API_BASE_URL}/me" >/dev/null
  auth_curl "${API_BASE_URL}/items" >/dev/null
  auth_curl "${API_BASE_URL}/tags" >/dev/null
}

check_text_capture_route() {
  local payload
  local status
  payload='{"plain_text":"route probe","tags":[],"html":null,"source_app":"smoke","source_device":null,"capture_method":"smoke","client_capture_id":"smoke-route-probe"}'

  echo "Checking text capture route"
  status="$(
    curl \
      --silent \
      --show-error \
      --output /dev/null \
      --write-out "%{http_code}" \
      -X POST \
      -H "Accept: application/json" \
      -H "Content-Type: application/json" \
      -d "${payload}" \
      "${API_BASE_URL}/items/text"
  )"

  if [[ "${status}" != "401" ]]; then
    echo "Text capture route returned HTTP ${status}; expected 401 before auth." >&2
    return 1
  fi
}

check_optional_capture() {
  if [[ -z "${LINKDROP_ACCESS_TOKEN:-}" || -z "${LINKDROP_SMOKE_CAPTURE_URL:-}" ]]; then
    echo "Skipping capture smoke; set LINKDROP_ACCESS_TOKEN and LINKDROP_SMOKE_CAPTURE_URL to enable it."
    return
  fi

  local escaped_url
  local payload
  local response
  escaped_url="$(json_escape "${LINKDROP_SMOKE_CAPTURE_URL}")"
  payload="{\"url\":\"${escaped_url}\"}"

  echo "Checking zero-tag capture"
  response="$(
    auth_curl \
      -X POST \
      -H "Content-Type: application/json" \
      -d "${payload}" \
      "${API_BASE_URL}/items"
  )"

  if [[ "${response}" != *'"item"'* || "${response}" != *'"copy_url"'* ]]; then
    echo "Capture smoke response did not include an item with copy_url." >&2
    return 1
  fi
}

check_health
check_text_capture_route
check_authenticated_reads
check_optional_capture

echo "Smoke checks passed."
