use argon2::{PasswordHasher, password_hash::phc::SaltString};
use axum::http::StatusCode;
use deadpool_postgres::Object;
use tokio_postgres::error::SqlState;
use uuid::Uuid;

use crate::api::{ApiError, db_error};

#[derive(Debug)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password: String,
    pub salt: String,
}

pub async fn find_by_email(db: &Object, email: &str) -> Result<Option<User>, ApiError> {
    let stmt = db
        .prepare_cached("SELECT id, username, email, password, salt FROM users WHERE email = $1")
        .await
        .map_err(db_error)?;

    let row = db.query_opt(&stmt, &[&email]).await.map_err(db_error)?;

    Ok(row.map(|r| User {
        id: r.get("id"),
        username: r.get("username"),
        email: r.get("email"),
        password: r.get("password"),
        salt: r.get("salt"),
    }))
}

pub async fn create(
    db: &Object,
    username: &str,
    email: &str,
    password: &str,
) -> Result<Uuid, ApiError> {
    let id = Uuid::now_v7();
    let salt = SaltString::generate();

    let password_owned = password.to_owned();
    let salt_clone = salt.clone();
    let pw = tokio::task::spawn_blocking(move || {
        let argon2 = crate::utils::get_argon();
        argon2
            .hash_password_with_salt(password_owned.as_bytes(), &salt_clone.to_salt())
            .expect("password hashing should not fail")
            .to_string()
    })
    .await
    .map_err(|e| {
        tracing::error!("spawn_blocking error in password hashing: {e}");
        ApiError::Rejection(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal server error".into(),
        )
    })?;

    let stmt = db
        .prepare_cached(
            "INSERT INTO users (id, username, email, password, salt) VALUES ($1, $2, $3, $4, $5)",
        )
        .await
        .map_err(db_error)?;

    db.execute(&stmt, &[&id, &username, &email, &pw, &salt.as_ref()])
        .await
        .map_err(|e| {
            let Some(db_err) = e.as_db_error() else {
                return db_error(e);
            };

            if *db_err.code() == SqlState::UNIQUE_VIOLATION {
                return match db_err.constraint() {
                    Some("users_username_key") => {
                        ApiError::Rejection(StatusCode::CONFLICT, "users already exists".into())
                    }
                    Some("users_email_key") => {
                        ApiError::Rejection(StatusCode::CONFLICT, "email already exists".into())
                    }
                    _ => ApiError::Rejection(StatusCode::CONFLICT, "already exists".into()),
                };
            }

            db_error(e)
        })?;

    Ok(id)
}
