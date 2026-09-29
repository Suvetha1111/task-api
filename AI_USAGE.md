# AI usage

## Tools

- **Claude (Anthropic), via the Claude desktop app in Cowork mode.** Used to
  scaffold the repository from the assignment prompt, generate the first
  version of every layer (models, repositories, services, managers,
  controllers, routes, middleware, containers), the migration, the
  `validate.sh` script, the integration test harness, the CI workflow and the
  first draft of this README.

No other AI tools were used.

## What was reviewed and changed by hand

- Verified the architecture against the assignment: one trait + one `Impl`
  per layer, `Arc<dyn Trait + Send + Sync + 'static>` in containers, a single
  `thiserror` `AppError` implementing Actix `ResponseError`, `?` propagation
  through `From<sqlx::Error>` / `From<redis::RedisError>`.
- Confirmed the auth middleware is strict (401 on missing / invalid / expired
  token and on a deleted user) rather than the optional-auth behaviour of the
  reference production service.
- Replaced a hard-coded dummy bcrypt hash (used for constant-time login on
  unknown emails) with one computed at service construction, so it can never
  be an invalid hash string.
- Checked the transaction boundaries: `verify-2fa` locks the challenge with
  `FOR UPDATE` and rolls back on a wrong code; assign/patch lock task rows,
  commit, then delete the Redis keys of every affected user.
- Ran `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
  and `scripts/validate.sh` against real Postgres 16 and Redis 7, including a
  manual check that stopping Redis makes `GET /tasks/view-my-tasks` return
  500 and that it recovers when Redis is back.
- Pasted the real James Bond responses from that validation run into the
  README.
