use argon2::{PasswordHasher, password_hash::phc::SaltString};
use async_trait::async_trait;
use deadpool_postgres::Object;
use std::sync::Arc;
use tokio_postgres::Row;
use tokio_postgres::error::SqlState;
use uuid::Uuid;

use crate::models::ModelError;
use crate::models::constant::{CREATE_USER_SQL, GET_USER_BY_EMAIL_SQL, GET_USER_BY_ID_SQL};
use crate::models::{FromRow, StandardModel, fetch_one};

#[derive(Debug)]
pub struct User {
    pub id: Option<Uuid>,
    pub username: Arc<str>,
    pub email: Arc<str>,
    pub password: Arc<str>,
    pub salt: Option<Arc<str>>,
}

impl User {
    #[must_use]
    pub fn new(email: &str, username: &str, password: &str) -> Self {
        Self {
            id: None,
            email: Arc::from(email),
            password: Arc::from(password),
            username: Arc::from(username),
            salt: None,
        }
    }

    /// Find a user by email.
    ///
    /// # Errors
    /// Returns [`ModelError::NotFound`] when the user does not exist,
    /// or [`ModelError::Database`] when the query fails.
    #[tracing::instrument(skip_all)]
    pub async fn get_by_email(db: &Object, email: &str) -> Result<Self, ModelError> {
        fetch_one(db, GET_USER_BY_EMAIL_SQL, &[&email], "user not found").await
    }

    /// Find a user by id.
    ///
    /// # Errors
    /// Returns [`ModelError::NotFound`] when the user does not exist,
    /// or [`ModelError::Database`] when the query fails.
    #[tracing::instrument(skip_all)]
    pub async fn get_by_id(db: &Object, id: Uuid) -> Result<Self, ModelError> {
        fetch_one(db, GET_USER_BY_ID_SQL, &[&id], "user not found").await
    }
}

#[async_trait]
impl StandardModel for User {
    type Id = Uuid;

    #[tracing::instrument(skip_all)]
    async fn create(&self, db: &Object) -> Result<Self::Id, ModelError> {
        let id = Uuid::now_v7();
        let salt = SaltString::generate();
        let salt_value = salt.to_string();

        let password = self.password.to_string();
        let hash = tokio::task::spawn_blocking(move || {
            let argon2 = crate::utils::get_argon();
            argon2
                .hash_password_with_salt(password.as_bytes(), &salt.to_salt())
                .map(|hash| hash.to_string())
        })
        .await
        .map_err(|e| ModelError::internal(format!("password hashing task failed: {e}")))?
        .map_err(|e| ModelError::internal(format!("password hashing failed: {e}")))?;

        let stmt = db
            .prepare_cached(CREATE_USER_SQL)
            .await
            .map_err(ModelError::from)?;

        let username: &str = &self.username;
        let email: &str = &self.email;

        db.execute(&stmt, &[&id, &username, &email, &hash, &salt_value])
            .await
            .map_err(user_write_error)?;

        Ok(id)
    }
}

/// Map a unique violation on `users` to a conflict error.
fn user_write_error(e: tokio_postgres::Error) -> ModelError {
    let Some(db_err) = e.as_db_error() else {
        return ModelError::Database(e);
    };

    if *db_err.code() == SqlState::UNIQUE_VIOLATION {
        return match db_err.constraint() {
            Some("users_username_key") => ModelError::conflict("username already exists"),
            Some("useUsedOrNotFoundrs_email_key") => ModelError::conflict("email already exists"),
            _ => ModelError::conflict("already exists"),
        };
    }

    ModelError::Database(e)
}

impl FromRow for User {
    fn from_row(r: &Row) -> Result<Self, ModelError> {
        let id: Uuid = r.try_get("id").map_err(ModelError::from)?;
        let username: String = r.try_get("username").map_err(ModelError::from)?;
        let email: String = r.try_get("email").map_err(ModelError::from)?;
        let password: String = r.try_get("password").map_err(ModelError::from)?;
        let salt: Option<String> = r.try_get("salt").map_err(ModelError::from)?;

        Ok(Self {
            id: Some(id),
            username: Arc::from(username),
            email: Arc::from(email),
            password: Arc::from(password),
            salt: salt.map(Arc::from),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_user_has_no_id_and_no_salt() {
        let user = User::new("user@example.com", "user", "secret");

        assert!(user.id.is_none());
        assert!(user.salt.is_none());
        assert_eq!(user.email.as_ref(), "user@example.com");
        assert_eq!(user.username.as_ref(), "user");
        assert_eq!(user.password.as_ref(), "secret");
    }
}
