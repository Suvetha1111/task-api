use std::sync::Arc;

use async_trait::async_trait;

use crate::error::AppError;
use crate::models::{NewUser, Role, SeedResponse};
use crate::repositories::UserRepository;

/// bcrypt cost. 10 keeps login and tests fast while still being a real hash.
pub const BCRYPT_COST: u32 = 10;

#[async_trait]
pub trait UserService: Send + Sync + 'static {
    /// Idempotent: creates the well-known users only when their email is missing.
    async fn seed_users(&self) -> Result<SeedResponse, AppError>;
}

pub struct UserServiceImpl {
    users: Arc<dyn UserRepository>,
}

impl UserServiceImpl {
    pub fn new(users: Arc<dyn UserRepository>) -> Self {
        Self { users }
    }
}

struct SeedUser {
    full_name: &'static str,
    email: &'static str,
    password: &'static str,
    role: Role,
}

const SEED_USERS: &[SeedUser] = &[
    SeedUser {
        full_name: "Admin",
        email: "admin@example.com",
        password: "AdminPass123!",
        role: Role::Admin,
    },
    SeedUser {
        full_name: "James Bond",
        email: "jamesbond@example.com",
        password: "BondPass123!",
        role: Role::Staff,
    },
];

#[async_trait]
impl UserService for UserServiceImpl {
    async fn seed_users(&self) -> Result<SeedResponse, AppError> {
        let mut created = Vec::new();
        let mut existing = Vec::new();
        for seed in SEED_USERS {
            let new_user = NewUser {
                full_name: seed.full_name.to_string(),
                email: seed.email.to_string(),
                hashed_password: bcrypt::hash(seed.password, BCRYPT_COST)?,
                role: seed.role,
            };
            if self.users.insert_if_missing(&new_user).await? {
                created.push(seed.email.to_string());
            } else {
                existing.push(seed.email.to_string());
            }
        }
        Ok(SeedResponse { created, existing })
    }
}
