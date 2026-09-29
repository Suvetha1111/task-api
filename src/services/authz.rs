//! Pure authorization rules.

use crate::error::AppError;
use crate::models::Role;

/// Only admins may pass. Staff get 403.
pub fn require_admin(role: Role) -> Result<(), AppError> {
    if role == Role::Admin {
        Ok(())
    } else {
        Err(AppError::Forbidden("admin role required".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_passes() {
        assert!(require_admin(Role::Admin).is_ok());
    }

    #[test]
    fn staff_is_forbidden() {
        match require_admin(Role::Staff) {
            Err(AppError::Forbidden(_)) => {}
            other => panic!("expected Forbidden, got {other:?}"),
        }
    }
}
