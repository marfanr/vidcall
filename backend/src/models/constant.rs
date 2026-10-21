// refresh tokens
pub const FIND_AND_MARK_SQL: &str = "UPDATE refresh_tokens SET used_at = $1 WHERE jti = $2 AND revoked_at IS NULL AND used_at IS NULL AND expires_at > $3 AND family_expires_at > $3 RETURNING user_id, family_id, expires_at, family_expires_at";
pub const CHECK_REUSE_AND_REVOKE_FAM_SQL: &str =
    "SELECT used_at, revoked_at FROM refresh_tokens WHERE jti = $1";

// users
pub const GET_USER_BY_EMAIL_SQL: &str =
    "SELECT id, username, email, password, salt FROM users WHERE email = $1";
pub const GET_USER_BY_ID_SQL: &str =
    "SELECT id, username, email, password, salt FROM users WHERE id = $1";
pub const CREATE_USER_SQL: &str =
    "INSERT INTO users (id, username, email, password, salt) VALUES ($1, $2, $3, $4, $5)";

// rooms
pub const CREATE_ROOM_SQL: &str = "INSERT INTO room (owner_id, alias, passcode, title, options)
                 VALUES ($1, $2, $3, COALESCE($4, 'Untitled Meeting'), COALESCE($5::jsonb, '{}'::jsonb))
                 RETURNING id";
pub const GET_ROOM_BY_ALIAS_SQL: &str =
    "SELECT id, alias, owner_id, passcode, title, options FROM room WHERE alias = $1";
