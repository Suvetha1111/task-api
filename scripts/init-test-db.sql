-- Runs once when the Postgres container is first created.
-- Creates the database used by `cargo test`.
CREATE DATABASE taskdb_test OWNER task;
