use std::{env, sync::OnceLock};

use argon2::{Algorithm, Argon2, Params, Version};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::de::DeserializeOwned;
use sonic_rs::{Deserialize, Serialize};
use uuid::Uuid;
use validator::ValidationError;

// Argon
pub fn get_argon<'a>() -> Argon2<'a> {
    Argon2::new_with_secret(
        b"bijisatu",
        Algorithm::Argon2i,
        Version::V0x13,
        Params::default(),
    )
    .expect("error to setup argon")
}

// JWT
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenUse {
    Access,
    #[default]
    Refresh,
}

pub const ISSUER: &str = "http://localhost:8080/";

pub trait JwtClaims: Serialize + DeserializeOwned {
    const AUDIENCE: &'static str;
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct AccessClaims {
    pub sub: String,
    pub exp: u64,
    pub iat: u64,
    pub iss: String,
    pub aud: String,
    pub token_use: TokenUse,
    #[serde(default)]
    pub scope: String,
}

#[derive(Default, Debug, sonic_rs::Serialize, sonic_rs::Deserialize)]
pub struct RefreshClaims {
    pub sub: String,
    pub exp: u64,
    pub iat: u64,
    pub iss: String,
    pub aud: String,
    pub token_use: TokenUse,
    pub jti: String,
    pub family_id: String,
}

impl JwtClaims for RefreshClaims {
    const AUDIENCE: &'static str = "auth/refresh";
}

impl JwtClaims for AccessClaims {
    const AUDIENCE: &'static str = "auth/access";
}

struct Keys {
    access_enc: EncodingKey,
    access_dec: DecodingKey,
    refresh_enc: EncodingKey,
    refresh_dec: DecodingKey,
}

static KEYS: OnceLock<Keys> = OnceLock::new();
pub const ACCESS_TTL: Duration = Duration::minutes(10);
pub const REFRESH_TTL: Duration = Duration::days(7);
pub const FAMILY_TTL: Duration = Duration::days(30);

fn load_secret(name: &str) -> String {
    let s = env::var(name).unwrap_or_else(|_| panic!("{name} must be configured on .env"));
    assert!(s.len() >= 32, "{name} must be at least 32 bytes");
    s
}

pub fn init_keys() {
    keys();
}

fn keys() -> &'static Keys {
    KEYS.get_or_init(|| {
        let access = load_secret("JWT_ACCESS_SECRET");
        let refresh = load_secret("JWT_REFRESH_SECRET");
        assert_ne!(access, refresh, "access and refresh secrets must differ");
        Keys {
            access_enc: EncodingKey::from_secret(access.as_bytes()),
            access_dec: DecodingKey::from_secret(access.as_bytes()),
            refresh_enc: EncodingKey::from_secret(refresh.as_bytes()),
            refresh_dec: DecodingKey::from_secret(refresh.as_bytes()),
        }
    })
}

#[derive(Debug, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

#[tracing::instrument()]
pub fn sign_pair(
    user_id: Uuid,
    scope: &str,
    jti: Uuid,
    family_id: Uuid,
) -> Result<TokenPair, jsonwebtoken::errors::Error> {
    let mut header = Header::new(jsonwebtoken::Algorithm::HS512);
    header.kid = Some("vidcall".to_string());
    let keys = keys();
    let now = Utc::now();

    let jti = jti.to_string();
    let family_id = family_id.to_string();

    let access = AccessClaims {
        sub: user_id.to_string(),
        exp: (now + ACCESS_TTL).timestamp() as u64,
        iat: now.timestamp() as u64,
        iss: ISSUER.to_owned(),
        aud: AccessClaims::AUDIENCE.to_owned(),
        token_use: TokenUse::Access,
        scope: scope.to_owned(),
    };

    let refresh = RefreshClaims {
        sub: user_id.to_string(),
        exp: (now + REFRESH_TTL).timestamp().cast_unsigned(),
        iat: now.timestamp() as u64,
        iss: ISSUER.to_owned(),
        aud: RefreshClaims::AUDIENCE.to_owned(),
        token_use: TokenUse::Refresh,
        jti,
        family_id,
    };

    Ok(TokenPair {
        access_token: encode(&header, &access, &keys.access_enc)?,
        refresh_token: encode(&header, &refresh, &keys.refresh_enc)?,
    })
}

#[tracing::instrument()]
pub fn verify_jwt<T: JwtClaims>(token: &str) -> Result<T, jsonwebtoken::errors::Error> {
    let key = keys();
    let mut val = Validation::new(jsonwebtoken::Algorithm::HS512);
    val.set_audience(&[T::AUDIENCE]);
    val.set_issuer(&[ISSUER]);
    val.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
    val.leeway = 5;

    let dec_key = if T::AUDIENCE == AccessClaims::AUDIENCE {
        &key.access_dec
    } else {
        &key.refresh_dec
    };

    decode::<T>(token, dec_key, &val).map(|d| d.claims)
}

// custom validator
pub fn validate_uuid(id: &str) -> Result<(), ValidationError> {
    Uuid::parse_str(id)
        .map_err(|_| ValidationError::new("invalid uuid"))
        .map(|_| ())
}
