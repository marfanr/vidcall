// refresh tokens
pub const FIND_AND_MARK_SQL: &str = "UPDATE refresh_tokens SET used_at = $1
        WHERE jti = $2 AND revoked_at IS NULL AND used_at IS NULL 
        AND expires_at > $3 AND family_expires_at > $3 
        RETURNING jti, user_id, family_id, expires_at, family_expires_at";
pub const CHECK_REUSE_AND_REVOKE_FAM_SQL: &str =
    "SELECT used_at, revoked_at FROM refresh_tokens WHERE jti = $1";
pub const REVOKE_FAMILY_SQL: &str =
    "UPDATE refresh_tokens SET revoked_at = $1 WHERE family_id = $2";
pub const REVOKE_SQL: &str = "UPDATE refresh_tokens SET revoked_at = $1 WHERE jti = $2";
pub const PUBLISH_REFRESH_TOKEN_SQL: &str = "INSERT INTO refresh_tokens (user_id, jti, family_id, expires_at, family_expires_at) VALUES ($1, $2, $3, $4, $5)";

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
    "SELECT r.id, r.alias, r.owner_id, r.passcode, r.title, r.options, u.username AS owner,  r.start_at
     FROM room r
     JOIN users u ON u.id = r.owner_id
     WHERE r.alias = $1";

pub const GET_ROOM_BY_ID_SQL: &str =
    "SELECT r.id, r.alias, r.owner_id, r.passcode, r.title, r.options, u.username AS owner, r.start_at
     FROM room r
     JOIN users u ON u.id = r.owner_id
     WHERE r.id = $1";
