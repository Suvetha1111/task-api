# AI usage

AI was used to move faster. I still own the design, the local setup, the
validation runs, and the bugs I found by hand.

## Tools

- **Claude (Anthropic), Cowork.** First pass of the repo from the assignment
  prompt: layers (routes, controllers, managers, services, repositories,
  containers), the Postgres migration, the first `validate.sh`, integration
  tests, GitHub Actions, and an early README draft.
- **Cursor.** Used while I compared the assignment to a production Actix
  service I already work with (`viome-backend`). I used that to check the
  layering, Redis cache style, `AppError`, and auth middleware. Cursor also
  helped add `POST /dev/reset` after I saw a real validation failure.

I did not paste the assignment into one prompt and submit the first output
unchanged.

## What I did myself

- Chose the stack and the folder layout to match how I work in Rust: Actix,
  SQLx, Redis, JWT only after 2FA, admin vs staff in the service layer, SQL
  only in repositories.
- Set up local Postgres and Redis, copied `.env.example`, ran migrations, and
  started the API with `cargo run`.
- Ran the required workflow myself: seed Admin and James Bond, login without a
  JWT, read the code from `/dev/email-logs/latest`, verify 2FA, create 5
  tasks, assign 3, confirm James Bond gets 403 on create and assign.
- Ran `scripts/validate.sh` against a live server. The first successful run
  produced the James Bond JSON that is pasted in the README (`cache.hit`
  false, then true).
- Hit `Address already in use` on port 8080 and traced it to another process
  still bound to that port, not an application crash.
- Re-ran validation and saw James Bond with **6** tasks instead of 3. I
  checked the response and the script: seed is idempotent, but tasks were
  appended. That is why `POST /dev/reset` exists. I confirmed the script now
  resets first so a second run still returns exactly 3 assigned tasks.
- Reviewed the generated code before keeping it:
  - login returns only `login_challenge_id`
  - `login_challenges` stores a bcrypt hash; `email_logs` is the local inbox
  - wrong / expired / reused codes are 401
  - staff cannot create or assign
  - view-my-tasks reads Postgres, then Redis; `cache.hit` is not stored
  - assign and patch delete `tasks:view:{user_id}` for every affected user
- Replaced a hard-coded dummy bcrypt hash with one built when the auth
  service starts, so unknown-email login still takes a similar amount of time.
- Ran `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and
  `cargo test` against `taskdb_test` and Redis. Stopped Redis once to confirm
  `GET /tasks/view-my-tasks` returns 500 instead of silently skipping the
  cache.
- Wrote and kept this file so a reviewer can see what AI produced and what I
  changed or verified.

## What I can explain without the tool

- Why the first login step must not return a JWT.
- Why the one-time code is hashed on the challenge row.
- How `GET /tasks/view-my-tasks` decides `cache.hit`.
- Why assigning a task must clear both the previous assignee and the new
  assignee.
- Why a second `validate.sh` run failed with 6 tasks before reset existed.
