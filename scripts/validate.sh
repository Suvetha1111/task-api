#!/usr/bin/env bash
# End-to-end validation of the task API against a running server.
# Exits non-zero on the first mismatch. Requires: curl, jq.
set -euo pipefail

BASE="${BASE_URL:-http://127.0.0.1:8080}"
JSON='content-type: application/json'
STEP=0

step() { STEP=$((STEP + 1)); echo "== [$STEP] $*"; }
fail() { echo "FAIL: $*" >&2; exit 1; }

# request METHOD PATH [BODY] [TOKEN] -> sets BODY_OUT and CODE_OUT
request() {
  local method="$1" path="$2" body="${3:-}" token="${4:-}"
  local args=(-s -o /tmp/validate_body.json -w '%{http_code}' -X "$method" "$BASE$path" -H "$JSON")
  [[ -n "$token" ]] && args+=(-H "Authorization: Bearer $token")
  [[ -n "$body" ]] && args+=(-d "$body")
  CODE_OUT="$(curl "${args[@]}")"
  BODY_OUT="$(cat /tmp/validate_body.json)"
}

expect_code() { [[ "$CODE_OUT" == "$1" ]] || fail "expected HTTP $1, got $CODE_OUT: $BODY_OUT"; }

login_and_verify() {
  local email="$1" password="$2"
  request POST /auth/login "{\"email\":\"$email\",\"password\":\"$password\"}"
  expect_code 200
  local challenge
  challenge="$(jq -r '.login_challenge_id // empty' <<<"$BODY_OUT")"
  [[ -n "$challenge" ]] || fail "login_challenge_id missing: $BODY_OUT"
  for key in access_token token jwt; do
    [[ "$(jq -r "has(\"$key\")" <<<"$BODY_OUT")" == "false" ]] || fail "login body must not contain $key"
  done
  request GET /dev/email-logs/latest
  expect_code 200
  local code
  code="$(jq -r '.verification_code' <<<"$BODY_OUT")"
  [[ "$code" =~ ^[0-9]{6}$ ]] || fail "verification_code is not 6 digits: $BODY_OUT"
  request POST /auth/verify-2fa "{\"login_challenge_id\":\"$challenge\",\"code\":\"$code\"}"
  expect_code 200
  TOKEN_OUT="$(jq -r '.access_token // empty' <<<"$BODY_OUT")"
  [[ -n "$TOKEN_OUT" ]] || fail "access_token missing: $BODY_OUT"
}

step "POST /dev/reset"
request POST /dev/reset
expect_code 200
[[ "$(jq -r '.reset' <<<"$BODY_OUT")" == "true" ]] || fail "reset flag missing: $BODY_OUT"

step "POST /seed/users"
request POST /seed/users
expect_code 200

step "Login as Admin (challenge only, no token)"
step "Read Admin code from /dev/email-logs/latest"
step "Verify 2FA, save Admin JWT"
login_and_verify admin@example.com 'AdminPass123!'
ADMIN="$TOKEN_OUT"

step "Create five tasks as Admin"
TASK_IDS=()
while IFS='|' read -r title priority; do
  request POST /tasks "{\"title\":\"$title\",\"description\":\"$title\",\"status\":\"todo\",\"priority\":\"$priority\"}" "$ADMIN"
  expect_code 201
  id="$(jq -r '.id // empty' <<<"$BODY_OUT")"
  [[ -n "$id" ]] || fail "task id missing: $BODY_OUT"
  TASK_IDS+=("$id")
done <<'EOF'
Review dossier|high
Check safe house|medium
File report|low
Archive notes|low
Schedule briefing|medium
EOF
[[ ${#TASK_IDS[@]} -eq 5 ]] || fail "expected 5 task ids"

step "Assign the first three tasks to jamesbond@example.com"
ASSIGN_IDS="$(printf '"%s",' "${TASK_IDS[@]:0:3}")"
ASSIGN_IDS="[${ASSIGN_IDS%,}]"
request POST /tasks/assign "{\"task_ids\":$ASSIGN_IDS,\"assignee_email\":\"jamesbond@example.com\"}" "$ADMIN"
expect_code 200

step "Login as James Bond, read his code, verify 2FA"
login_and_verify jamesbond@example.com 'BondPass123!'
BOND="$TOKEN_OUT"

step "POST /tasks as James Bond -> 403"
request POST /tasks '{"title":"x","description":"x","status":"todo","priority":"low"}' "$BOND"
expect_code 403

step "POST /tasks/assign as James Bond -> 403"
request POST /tasks/assign "{\"task_ids\":[\"${TASK_IDS[3]}\"],\"assignee_email\":\"jamesbond@example.com\"}" "$BOND"
expect_code 403

step "GET /tasks/view-my-tasks as James Bond (cache miss)"
request GET /tasks/view-my-tasks "" "$BOND"
expect_code 200
FIRST="$BODY_OUT"
echo "$FIRST" | jq .
[[ "$(jq -r '.user.email' <<<"$FIRST")" == "jamesbond@example.com" ]] || fail "user.email"
[[ "$(jq -r '.user.role' <<<"$FIRST")" == "staff" ]] || fail "user.role"
[[ "$(jq -r '.tasks | length' <<<"$FIRST")" == "3" ]] || fail "expected exactly 3 tasks"
[[ "$(jq -r '[.tasks[].priority] | join(",")' <<<"$FIRST")" == "high,medium,low" ]] || fail "priority order"
[[ "$(jq -r '[.tasks[].assigned_to] | unique | join(",")' <<<"$FIRST")" == "jamesbond@example.com" ]] || fail "assigned_to"
[[ "$(jq -r '.summary.total_assigned_tasks' <<<"$FIRST")" == "3" ]] || fail "summary.total_assigned_tasks"
[[ "$(jq -r '.cache.hit' <<<"$FIRST")" == "false" ]] || fail "first view must be a cache miss"
EXPECTED_IDS="$(printf '%s\n' "${TASK_IDS[@]:0:3}" | sort | tr '\n' ',')"
ACTUAL_IDS="$(jq -r '.tasks[].id' <<<"$FIRST" | sort | tr '\n' ',')"
[[ "$EXPECTED_IDS" == "$ACTUAL_IDS" ]] || fail "task ids differ: expected $EXPECTED_IDS got $ACTUAL_IDS"

step "GET /tasks/view-my-tasks again (cache hit)"
request GET /tasks/view-my-tasks "" "$BOND"
expect_code 200
SECOND="$BODY_OUT"
echo "$SECOND" | jq .
[[ "$(jq -r '.cache.hit' <<<"$SECOND")" == "true" ]] || fail "second view must be a cache hit"
[[ "$(jq -r '.tasks[].id' <<<"$SECOND" | sort | tr '\n' ',')" == "$ACTUAL_IDS" ]] || fail "second view returned different ids"
[[ "$(jq -c 'del(.cache)' <<<"$FIRST")" == "$(jq -c 'del(.cache)' <<<"$SECOND")" ]] || fail "cached body differs from first body"

echo
echo "ALL CHECKS PASSED"
