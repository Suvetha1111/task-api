//! Containers build each feature's repository -> service -> manager chain and
//! expose the result as `Arc<dyn Trait + Send + Sync + 'static>`.

pub mod auth_container;
pub mod task_container;
pub mod user_container;

pub use auth_container::AuthContainer;
pub use task_container::TaskContainer;
pub use user_container::UserContainer;
