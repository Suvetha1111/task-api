use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{Claims, Role};

/// Issues and verifies HS256 access tokens.
pub trait TokenService: Send + Sync + 'static {
    fn issue(&self, user_id: Uuid, email: &str, role: Role) -> Result<String, AppError>;
    /// Returns `Unauthorized` for a bad signature, malformed token or expired `exp`.
    fn verify(&self, token: &str) -> Result<Claims, AppError>;
}

#[derive(Clone)]
pub struct TokenServiceImpl {
    encoding: EncodingKey,
    decoding: DecodingKey,
    expiry_secs: i64,
}

impl TokenServiceImpl {
    pub fn new(secret: &str, expiry_secs: u64) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
            expiry_secs: i64::try_from(expiry_secs).unwrap_or(i64::MAX),
        }
    }
}

impl TokenService for TokenServiceImpl {
    fn issue(&self, user_id: Uuid, email: &str, role: Role) -> Result<String, AppError> {
        let now = Utc::now();
        let claims = Claims {
            sub: user_id,
            email: email.to_string(),
            role,
            iat: now.timestamp(),
            exp: (now + Duration::seconds(self.expiry_secs)).timestamp(),
        };
        Ok(encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &self.encoding,
        )?)
    }

    fn verify(&self, token: &str) -> Result<Claims, AppError> {
        let validation = Validation::new(Algorithm::HS256);
        decode::<Claims>(token, &self.decoding, &validation)
            .map(|data| data.claims)
            .map_err(|_| AppError::Unauthorized("invalid or expired token".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let svc = TokenServiceImpl::new("secret", 60);
        let id = Uuid::new_v4();
        let token = svc.issue(id, "a@example.com", Role::Staff).unwrap();
        let claims = svc.verify(&token).unwrap();
        assert_eq!(claims.sub, id);
        assert_eq!(claims.email, "a@example.com");
        assert_eq!(claims.role, Role::Staff);
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn wrong_secret_is_unauthorized() {
        let a = TokenServiceImpl::new("a", 60);
        let b = TokenServiceImpl::new("b", 60);
        let token = a
            .issue(Uuid::new_v4(), "x@example.com", Role::Admin)
            .unwrap();
        assert!(matches!(b.verify(&token), Err(AppError::Unauthorized(_))));
    }
}
