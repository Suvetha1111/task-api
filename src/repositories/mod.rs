//! Repositories run SQL and nothing else. No business rules live here.

pub mod email_log_repository;
pub mod login_challenge_repository;
pub mod task_repository;
pub mod user_repository;

pub use email_log_repository::{EmailLogRepository, PgEmailLogRepository};
pub use login_challenge_repository::{LoginChallengeRepository, PgLoginChallengeRepository};
pub use task_repository::{PgTaskRepository, TaskRepository};
pub use user_repository::{PgUserRepository, UserRepository};

/// A Postgres transaction handle passed into repository methods that must run
/// inside a caller-controlled transaction.
pub type Tx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;
