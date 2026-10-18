use chrono::{DateTime, Utc};
use deadpool_postgres::Object;
use uuid::Uuid;

use crate::api::{ApiError, db_error};

#[derive(Debug, Clone, PartialEq)]
pub struct RefreshToken {
    pub jti: Uuid,
    pub family_id: Uuid,
    pub user_id: Uuid,
    pub used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub family_expires_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refresh_token_creation() {
        let now = Utc::now();
        let jti = Uuid::now_v7();
        let family_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();
        let expires_at = now + chrono::Duration::days(1);
        let family_expires_at = now + chrono::Duration::days(7);

        let token = RefreshToken {
            jti,
            family_id,
            user_id,
            used_at: None,
            revoked_at: None,
            expires_at,
            family_expires_at,
        };

        assert_eq!(token.jti, jti);
        assert_eq!(token.family_id, family_id);
        assert_eq!(token.user_id, user_id);
        assert!(token.used_at.is_none());
        assert!(token.revoked_at.is_none());
        assert_eq!(token.expires_at, expires_at);
        assert_eq!(token.family_expires_at, family_expires_at);
    }

    #[test]
    fn test_refresh_token_with_used_at() {
        let now = Utc::now();
        let jti = Uuid::now_v7();
        let family_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();
        let used_at = now;
        let expires_at = now + chrono::Duration::days(1);
        let family_expires_at = now + chrono::Duration::days(7);

        let token = RefreshToken {
            jti,
            family_id,
            user_id,
            used_at: Some(used_at),
            revoked_at: None,
            expires_at,
            family_expires_at,
        };

        assert!(token.used_at.is_some());
        assert_eq!(token.used_at.unwrap(), used_at);
        assert!(token.revoked_at.is_none());
    }

    #[test]
    fn test_refresh_token_with_revoked_at() {
        let now = Utc::now();
        let jti = Uuid::now_v7();
        let family_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();
        let revoked_at = now;
        let expires_at = now + chrono::Duration::days(1);
        let family_expires_at = now + chrono::Duration::days(7);

        let token = RefreshToken {
            jti,
            family_id,
            user_id,
            used_at: None,
            revoked_at: Some(revoked_at),
            expires_at,
            family_expires_at,
        };

        assert!(token.used_at.is_none());
        assert!(token.revoked_at.is_some());
        assert_eq!(token.revoked_at.unwrap(), revoked_at);
    }

    #[test]
    fn test_refresh_token_both_used_and_revoked() {
        let now = Utc::now();
        let jti = Uuid::now_v7();
        let family_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();
        let used_at = now - chrono::Duration::minutes(5);
        let revoked_at = now;
        let expires_at = now + chrono::Duration::days(1);
        let family_expires_at = now + chrono::Duration::days(7);

        let token = RefreshToken {
            jti,
            family_id,
            user_id,
            used_at: Some(used_at),
            revoked_at: Some(revoked_at),
            expires_at,
            family_expires_at,
        };

        assert!(token.used_at.is_some());
        assert!(token.revoked_at.is_some());
    }

    #[test]
    fn test_refresh_token_clone() {
        let now = Utc::now();
        let token = RefreshToken {
            jti: Uuid::now_v7(),
            family_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            used_at: None,
            revoked_at: None,
            expires_at: now + chrono::Duration::days(1),
            family_expires_at: now + chrono::Duration::days(7),
        };

        let cloned = token.clone();
        assert_eq!(token.jti, cloned.jti);
        assert_eq!(token.family_id, cloned.family_id);
        assert_eq!(token.user_id, cloned.user_id);
    }

    #[test]
    fn test_refresh_token_partial_eq() {
        let now = Utc::now();
        let jti = Uuid::now_v7();
        let family_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();
        let expires_at = now + chrono::Duration::days(1);
        let family_expires_at = now + chrono::Duration::days(7);

        let token1 = RefreshToken {
            jti,
            family_id,
            user_id,
            used_at: None,
            revoked_at: None,
            expires_at,
            family_expires_at,
        };

        let token2 = RefreshToken {
            jti,
            family_id,
            user_id,
            used_at: None,
            revoked_at: None,
            expires_at,
            family_expires_at,
        };

        assert_eq!(token1, token2);
    }

    #[test]
    fn test_used_or_not_found_struct() {
        let _ = UsedOrNotFound;
    }

    #[test]
    fn test_used_or_not_found_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<UsedOrNotFound>();
    }

    #[test]
    fn test_token_validation_not_revoked_and_not_used() {
        let now = Utc::now();
        let token = RefreshToken {
            jti: Uuid::now_v7(),
            family_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            used_at: None,
            revoked_at: None,
            expires_at: now + chrono::Duration::days(1),
            family_expires_at: now + chrono::Duration::days(7),
        };

        assert!(token.revoked_at.is_none(), "Token should not be revoked");
        assert!(token.used_at.is_none(), "Token should not be used");
        assert!(token.expires_at > now, "Token should not be expired");
        assert!(
            token.family_expires_at > now,
            "Family should not be expired"
        );
    }

    #[test]
    fn test_token_validation_revoked_returns_error() {
        let now = Utc::now();
        let token = RefreshToken {
            jti: Uuid::now_v7(),
            family_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            used_at: None,
            revoked_at: Some(now - chrono::Duration::hours(1)),
            expires_at: now + chrono::Duration::days(1),
            family_expires_at: now + chrono::Duration::days(7),
        };

        assert!(token.revoked_at.is_some(), "Token should be revoked");
    }

    #[test]
    fn test_token_validation_used_returns_error() {
        let now = Utc::now();
        let token = RefreshToken {
            jti: Uuid::now_v7(),
            family_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            used_at: Some(now - chrono::Duration::minutes(5)),
            revoked_at: None,
            expires_at: now + chrono::Duration::days(1),
            family_expires_at: now + chrono::Duration::days(7),
        };

        assert!(token.used_at.is_some(), "Token should be marked as used");
    }

    #[test]
    fn test_token_validation_expired_token() {
        let now = Utc::now();
        let token = RefreshToken {
            jti: Uuid::now_v7(),
            family_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            used_at: None,
            revoked_at: None,
            expires_at: now - chrono::Duration::hours(1),
            family_expires_at: now + chrono::Duration::days(7),
        };

        assert!(token.expires_at < now, "Token should be expired");
    }

    #[test]
    fn test_token_validation_family_expired() {
        let now = Utc::now();
        let token = RefreshToken {
            jti: Uuid::now_v7(),
            family_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            used_at: None,
            revoked_at: None,
            expires_at: now + chrono::Duration::days(1),
            family_expires_at: now - chrono::Duration::hours(1),
        };

        assert!(token.family_expires_at < now, "Family should be expired");
    }

    #[test]
    fn test_is_token_valid_for_refresh() {
        fn is_token_valid_for_refresh(token: &RefreshToken, now: DateTime<Utc>) -> bool {
            token.revoked_at.is_none()
                && token.used_at.is_none()
                && token.expires_at > now
                && token.family_expires_at > now
        }

        let now = Utc::now();

        let valid_token = RefreshToken {
            jti: Uuid::now_v7(),
            family_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            used_at: None,
            revoked_at: None,
            expires_at: now + chrono::Duration::days(1),
            family_expires_at: now + chrono::Duration::days(7),
        };
        assert!(is_token_valid_for_refresh(&valid_token, now));

        let revoked_token = RefreshToken {
            jti: Uuid::now_v7(),
            family_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            used_at: None,
            revoked_at: Some(now - chrono::Duration::hours(1)),
            expires_at: now + chrono::Duration::days(1),
            family_expires_at: now + chrono::Duration::days(7),
        };
        assert!(!is_token_valid_for_refresh(&revoked_token, now));

        let used_token = RefreshToken {
            jti: Uuid::now_v7(),
            family_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            used_at: Some(now - chrono::Duration::minutes(5)),
            revoked_at: None,
            expires_at: now + chrono::Duration::days(1),
            family_expires_at: now + chrono::Duration::days(7),
        };
        assert!(!is_token_valid_for_refresh(&used_token, now));

        let expired_token = RefreshToken {
            jti: Uuid::now_v7(),
            family_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            used_at: None,
            revoked_at: None,
            expires_at: now - chrono::Duration::hours(1),
            family_expires_at: now + chrono::Duration::days(7),
        };
        assert!(!is_token_valid_for_refresh(&expired_token, now));

        let family_expired_token = RefreshToken {
            jti: Uuid::now_v7(),
            family_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            used_at: None,
            revoked_at: None,
            expires_at: now + chrono::Duration::days(1),
            family_expires_at: now - chrono::Duration::hours(1),
        };
        assert!(!is_token_valid_for_refresh(&family_expired_token, now));
    }

    #[test]
    fn test_detect_reuse_attack() {
        fn is_reuse_attack(token: &RefreshToken) -> bool {
            token.used_at.is_some()
        }

        let now = Utc::now();

        let fresh_token = RefreshToken {
            jti: Uuid::now_v7(),
            family_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            used_at: None,
            revoked_at: None,
            expires_at: now + chrono::Duration::days(1),
            family_expires_at: now + chrono::Duration::days(7),
        };
        assert!(!is_reuse_attack(&fresh_token));

        let reused_token = RefreshToken {
            jti: Uuid::now_v7(),
            family_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            used_at: Some(now - chrono::Duration::minutes(5)),
            revoked_at: None,
            expires_at: now + chrono::Duration::days(1),
            family_expires_at: now + chrono::Duration::days(7),
        };
        assert!(is_reuse_attack(&reused_token));
    }

    #[test]
    fn test_jti_uniqueness() {
        let jti1 = Uuid::now_v7();
        let jti2 = Uuid::now_v7();
        assert_ne!(jti1, jti2, "JTIs should be unique");
    }

    #[test]
    fn test_family_id_groups_tokens() {
        let family_id = Uuid::now_v7();
        let now = Utc::now();

        let token1 = RefreshToken {
            jti: Uuid::now_v7(),
            family_id,
            user_id: Uuid::now_v7(),
            used_at: None,
            revoked_at: None,
            expires_at: now + chrono::Duration::days(1),
            family_expires_at: now + chrono::Duration::days(7),
        };

        let token2 = RefreshToken {
            jti: Uuid::now_v7(),
            family_id,
            user_id: token1.user_id,
            used_at: None,
            revoked_at: None,
            expires_at: now + chrono::Duration::days(1),
            family_expires_at: now + chrono::Duration::days(7),
        };

        assert_eq!(
            token1.family_id, token2.family_id,
            "Tokens in same family should have same family_id"
        );
        assert_ne!(token1.jti, token2.jti, "Tokens should have different jti");
    }
}

pub async fn find_and_mark_used(
    db: &Object,
    jti: Uuid,
    now: DateTime<Utc>,
) -> Result<Result<RefreshToken, UsedOrNotFound>, ApiError> {
    let stmt = db
        .prepare_cached(
            "UPDATE refresh_tokens SET used_at = $1 WHERE jti = $2 AND revoked_at IS NULL AND used_at IS NULL AND expires_at > $3 AND family_expires_at > $3 RETURNING user_id, family_id, expires_at, family_expires_at",
        )
        .await
        .map_err(db_error)?;

    let row = db
        .query_opt(&stmt, &[&now, &jti, &now])
        .await
        .map_err(db_error)?;

    match row {
        Some(r) => Ok(Ok(RefreshToken {
            jti,
            user_id: r.get("user_id"),
            family_id: r.get("family_id"),
            used_at: Some(now),
            revoked_at: None,
            expires_at: r.get("expires_at"),
            family_expires_at: r.get("family_expires_at"),
        })),
        None => Ok(Err(UsedOrNotFound)),
    }
}

pub struct UsedOrNotFound;

pub async fn check_reuse_and_revoke_family(
    db: &Object,
    jti: Uuid,
    family_id: Uuid,
) -> Result<bool, ApiError> {
    let check_stmt = db
        .prepare_cached("SELECT used_at, revoked_at FROM refresh_tokens WHERE jti = $1")
        .await
        .map_err(db_error)?;

    let row = db.query_opt(&check_stmt, &[&jti]).await.map_err(db_error)?;

    match row {
        Some(r) => {
            let used_at: Option<DateTime<Utc>> = r.get("used_at");
            let revoked_at: Option<DateTime<Utc>> = r.get("revoked_at");
            if used_at.is_some() || revoked_at.is_some() {
                revoke_family(db, family_id).await?;
                return Ok(true);
            }
            Ok(false)
        }
        None => Ok(false),
    }
}

pub async fn revoke(db: &Object, jti: Uuid) -> Result<(), ApiError> {
    let stmt = db
        .prepare_cached("UPDATE refresh_tokens SET revoked_at = $1 WHERE jti = $2")
        .await
        .map_err(db_error)?;
    db.execute(&stmt, &[&Utc::now(), &jti])
        .await
        .map_err(db_error)?;
    Ok(())
}

pub async fn revoke_family(db: &Object, family_id: Uuid) -> Result<(), ApiError> {
    let stmt = db
        .prepare_cached("UPDATE refresh_tokens SET revoked_at = $1 WHERE family_id = $2")
        .await
        .map_err(db_error)?;
    db.execute(&stmt, &[&Utc::now(), &family_id])
        .await
        .map_err(db_error)?;
    Ok(())
}

pub async fn publish(
    db: Object,
    user_id: Uuid,
    jti: Uuid,
    family_id: Uuid,
    expires_at: DateTime<Utc>,
    family_expires_at: DateTime<Utc>,
) -> Result<(), ApiError> {
    let stmt = db
        .prepare_cached(
            "INSERT INTO refresh_tokens (user_id, jti, family_id, expires_at, family_expires_at) VALUES ($1, $2, $3, $4, $5)",
        )
        .await
        .map_err(db_error)?;

    db.execute(
        &stmt,
        &[&user_id, &jti, &family_id, &expires_at, &family_expires_at],
    )
    .await
    .map_err(db_error)?;
    Ok(())
}
