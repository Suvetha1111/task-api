# Task API

A small task-management HTTP API written in Rust. It has two roles (`admin`,
`staff`), a two-step login (password, then a 6-digit code delivered to a local
"inbox"), JWT bearer authentication, and a Redis cache in front of the
"my tasks" view with explicit invalidation on assign and update.

Stack: Actix-web 4, Tokio, SQLx 0.7 + PostgreSQL 16, Redis 7 (deadpool-redis),
serde + validator, thiserror, async-trait, bcrypt, jsonwebtoken, utoipa +
Swagger UI, tracing.

## Prerequisites

- Rust stable (edition 2021), `cargo`
- Docker with Compose (for Postgres and Redis)
- `sqlx-cli`: `cargo install sqlx-cli --no-default-features --features postgres,rustls`
- `curl` and `jq` for the validation script

## Run it

```bash
docker compose up -d                 # Postgres 16 (taskdb + taskdb_test) and Redis 7
cp .env.example .env                 # edit if your ports differ
sqlx migrate run                     # applies migrations/ to DATABASE_URL
cargo run                            # http://127.0.0.1:8080
```

Swagger UI: http://127.0.0.1:8080/swagger-ui/ (no token needed).
OpenAPI JSON: http://127.0.0.1:8080/api-docs/openapi.json

## Seed and validate

The fastest way is the script, which performs the whole flow below and exits
non-zero on any mismatch. It starts with `POST /dev/reset` so a second run
still sees exactly 3 tasks for James Bond:

```bash
scripts/validate.sh
```

Port 8080 must be this process. If `cargo run` prints `Address already in use`,
stop the other listener first (`lsof -nP -iTCP:8080 -sTCP:LISTEN`).

The same flow by hand:

```bash
BASE=http://127.0.0.1:8080
H='content-type: application/json'

# 0. wipe tasks/challenges/email logs from a previous run (keeps users)
curl -s -X POST $BASE/dev/reset

# 1. seed (idempotent)
curl -s -X POST $BASE/seed/users

# 2. login as Admin -> challenge id only, no token
curl -s -X POST $BASE/auth/login -H "$H" \
  -d '{"email":"admin@example.com","password":"AdminPass123!"}'
# {"login_challenge_id":"<uuid>"}

# 3. read the code from the dev inbox
curl -s $BASE/dev/email-logs/latest
# {"to_email":"admin@example.com","subject":"Your verification code","verification_code":"123456","created_at":"..."}

# 4. verify 2FA -> JWT
curl -s -X POST $BASE/auth/verify-2fa -H "$H" \
  -d '{"login_challenge_id":"<uuid>","code":"123456"}'
# {"access_token":"<jwt>","token_type":"Bearer"}
ADMIN=<jwt>

# 5. create five tasks as Admin (repeat for each)
curl -s -X POST $BASE/tasks -H "$H" -H "Authorization: Bearer $ADMIN" \
  -d '{"title":"Review dossier","description":"desc","status":"todo","priority":"high"}'
#   "Check safe house" / medium, "File report" / low, "Archive notes" / low, "Schedule briefing" / medium

# 6. assign the first three to James Bond
curl -s -X POST $BASE/tasks/assign -H "$H" -H "Authorization: Bearer $ADMIN" \
  -d '{"task_ids":["<id1>","<id2>","<id3>"],"assignee_email":"jamesbond@example.com"}'

# 7. login as James Bond (steps 2-4 with jamesbond@example.com / BondPass123!)
BOND=<jwt>

# 8-9. staff is forbidden
curl -s -o /dev/null -w '%{http_code}\n' -X POST $BASE/tasks -H "$H" -H "Authorization: Bearer $BOND" \
  -d '{"title":"x","description":"x","status":"todo","priority":"low"}'      # 403
curl -s -o /dev/null -w '%{http_code}\n' -X POST $BASE/tasks/assign -H "$H" -H "Authorization: Bearer $BOND" \
  -d '{"task_ids":["<id4>"],"assignee_email":"jamesbond@example.com"}'       # 403

# 10. first view -> cache miss
curl -s $BASE/tasks/view-my-tasks -H "Authorization: Bearer $BOND"

# 11. second view -> cache hit
curl -s $BASE/tasks/view-my-tasks -H "Authorization: Bearer $BOND"
```

## Tests

```bash
cargo test
```

Integration tests in `tests/` boot the real application on a random port
against **`taskdb_test`** (created automatically by the Compose Postgres
container from `scripts/init-test-db.sql`) and the same Redis. Nothing is
mocked. They read `TEST_DATABASE_URL` (default
`postgres://task:task@localhost:5432/taskdb_test`) and `REDIS_URL`. Each test
truncates the tables and clears `tasks:view:*` keys, and tests are serialised
with a mutex so they can share one database. The expiry test builds the app
with `OTP_TTL_SECONDS=1` directly (no env change needed).

Unit tests cover the pure pieces: TTL/expiry math, cache key format, the admin
role check, and JWT issue/verify.

Lint the way CI does:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

## Design

### Layers

```
routes -> controllers -> managers -> services -> repositories
                 \_________ containers build the graph, AppState holds it
```

- **routes** (`src/routes`) map URLs to handlers. Protected routes live in one
  `/tasks` scope wrapped with the auth middleware.
- **controllers** (`src/controllers`) parse the HTTP request, validate the body,
  read the authenticated user from request extensions, call exactly one
  manager, and return `Result<HttpResponse, AppError>`; `AppError`'s
  `ResponseError` impl maps the error to a status code.
- **managers** (`src/managers`) orchestrate one use case each (login,
  verify-2fa, seed, create/assign/patch/view tasks).
- **services** (`src/services`) hold the business rules: the admin role check,
  the 2FA state machine, JWT issuing, and the cache hit/miss + invalidation
  logic.
- **repositories** (`src/repositories`) run SQL only. Methods that must be part
  of a caller-controlled transaction take `&mut Transaction`.
- **containers** (`src/containers`) construct each chain as
  `Arc<dyn Trait + Send + Sync + 'static>` and store it on a cloneable
  `AppState`.

Errors: `AppError` (`src/error.rs`) is a `thiserror` enum with
`BadRequest 400`, `Unauthorized 401`, `Forbidden 403`, `NotFound 404`,
`Database 500`, `Redis 500`, `Internal 500`. `From<sqlx::Error>` and
`From<redis::RedisError>` let every layer use `?`. There is no `unwrap` or
`expect` on the request path.

Authentication: the middleware (`src/middleware/auth.rs`) reads
`Authorization: Bearer <jwt>`, verifies the HS256 signature and `exp`, loads
the user from Postgres, and inserts `AuthUser { user_id, email, role }` into
request extensions. Missing, malformed, expired, or orphaned tokens are all
401. Public routes (`/seed/users`, `/auth/*`, `/dev/*`, Swagger) are outside
that scope.

### Why login does not return a JWT

Login proves only the first factor (something you know). Returning a token at
that point would make the second factor optional. So `POST /auth/login`
returns only a `login_challenge_id`; the JWT is issued by
`POST /auth/verify-2fa`, which consumes the challenge in one transaction:
the row is locked (`SELECT ... FOR UPDATE`), checked for `used_at IS NULL`,
checked for `now < expires_at`, bcrypt-verified, then marked used. A wrong
code rolls the transaction back, so the challenge stays usable; a reused or
expired code is rejected with a specific message.

### Why `login_challenges` stores a hash and `email_logs` stores the raw code

`login_challenges` is the security-relevant table: if it leaked, nobody should
be able to complete a login from it, so it holds only a bcrypt hash of the
code (like a password). `email_logs` stands in for the outbound email
provider during local development: it is the "inbox" that
`GET /dev/email-logs/latest` reads so the flow can be automated without SMTP.
It is the only place the raw code is persisted, and that endpoint answers 404
unless `APP_ENV=development`. The code is also logged once with `tracing` at
info level; passwords are never logged.

### Cache key and invalidation

- Key: `tasks:view:{user_id}`, TTL 60 seconds (`SETEX`).
- Value: JSON of `{ user, tasks, summary }`. `cache.hit` is added by the
  handler and is never stored.
- Miss: query Postgres (tasks joined with the assignee's email, ordered by
  `created_at, id`), write the payload, respond with `cache.hit = false`.
- Hit: deserialize and respond with `cache.hit = true` without touching
  Postgres. A raw SQL update in between is therefore invisible until the key
  is gone (the tests prove this by changing a title behind the API).
- Invalidation: `POST /tasks/assign` and `PATCH /tasks/{id}` delete the key of
  every affected user, the new assignee and any previous assignee, before
  returning. The next view is a miss and shows fresh data.
- If Redis is unreachable the request fails with 500 (`cache_error`). The
  cache is never silently bypassed.

## Real responses from `scripts/validate.sh`

James Bond, first `GET /tasks/view-my-tasks` (cache miss):

```json
{
  "user": {
    "email": "jamesbond@example.com",
    "role": "staff"
  },
  "tasks": [
    {
      "id": "32a6cdf1-1cc1-45a0-a802-9cdc9fc1b0e1",
      "title": "Review dossier",
      "status": "todo",
      "priority": "high",
      "assigned_to": "jamesbond@example.com"
    },
    {
      "id": "37559844-32b5-4125-871a-2ef6841bde41",
      "title": "Check safe house",
      "status": "todo",
      "priority": "medium",
      "assigned_to": "jamesbond@example.com"
    },
    {
      "id": "b76f2e2d-41cf-41c2-abc9-bc55ca124468",
      "title": "File report",
      "status": "todo",
      "priority": "low",
      "assigned_to": "jamesbond@example.com"
    }
  ],
  "summary": {
    "total_assigned_tasks": 3
  },
  "cache": {
    "hit": false
  }
}
```

James Bond, second `GET /tasks/view-my-tasks` (cache hit, same three ids):

```json
{
  "user": {
    "email": "jamesbond@example.com",
    "role": "staff"
  },
  "tasks": [
    {
      "id": "32a6cdf1-1cc1-45a0-a802-9cdc9fc1b0e1",
      "title": "Review dossier",
      "status": "todo",
      "priority": "high",
      "assigned_to": "jamesbond@example.com"
    },
    {
      "id": "37559844-32b5-4125-871a-2ef6841bde41",
      "title": "Check safe house",
      "status": "todo",
      "priority": "medium",
      "assigned_to": "jamesbond@example.com"
    },
    {
      "id": "b76f2e2d-41cf-41c2-abc9-bc55ca124468",
      "title": "File report",
      "status": "todo",
      "priority": "low",
      "assigned_to": "jamesbond@example.com"
    }
  ],
  "summary": {
    "total_assigned_tasks": 3
  },
  "cache": {
    "hit": true
  }
}
```

## Configuration

`.env.example`:

```
DATABASE_URL=postgres://task:task@localhost:5432/taskdb
REDIS_URL=redis://127.0.0.1:6379
JWT_SECRET=dev-only-change-me
JWT_EXPIRY_SECS=3600
OTP_TTL_SECONDS=300
APP_ENV=development
SERVER_HOST=127.0.0.1
SERVER_PORT=8080
```

`.env` is gitignored. `OTP_TTL_SECONDS` defaults to 300.

## Repository layout

```
migrations/            SQL migrations (sqlx migrate run)
scripts/validate.sh    end-to-end curl validation
scripts/init-test-db.sql  creates taskdb_test in the Compose container
src/app.rs             build_state() + configure(): shared by main and tests
src/config.rs          env -> Config
src/error.rs           AppError + ResponseError
src/models/            DTOs, DB rows, enums
src/repositories/      SQL
src/services/          business rules
src/managers/          use cases
src/controllers/       HTTP handlers
src/routes/            URL mapping
src/middleware/auth.rs bearer-token middleware
src/containers/        DI wiring
src/state.rs           AppState
src/swagger.rs         OpenAPI document
tests/                 integration tests (real Postgres + Redis)
.github/workflows/ci.yml  fmt, clippy -D warnings, test
```
