use std::env;

use argon2::{Algorithm, Argon2, Params, Version};
use jsonwebtoken::{
    DecodingKey, EncodingKey, Header, Validation, decode,
    jws::{Jws, encode},
};
use serde::de::DeserializeOwned;

// Argon
pub fn get_argon<'a>() -> Argon2<'a> {
    Argon2::new_with_secret(
        b"haiya",
        Algorithm::Argon2id,
        Version::V0x13,
        Params::default(),
    )
    .expect("error to setup argon")
}

// JWT
pub trait JwtClaims: sonic_rs::Serialize + DeserializeOwned {}

#[derive(Default, Debug, sonic_rs::Serialize, sonic_rs::Deserialize)]
pub struct AuthClaims {
    pub sub: String,
    pub exp: u64,
    pub iat: u64,
    pub id: String,
}

impl JwtClaims for AuthClaims {}

pub fn create_jwt<T: JwtClaims>(c: &T) -> Result<Jws<T>, jsonwebtoken::errors::Error> {
    let mut header = Header::new(jsonwebtoken::Algorithm::HS256);
    header.kid = Some("vidcall".to_string());

    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be configured on .env");
    let key = EncodingKey::from_secret(secret.as_bytes());

    let token = encode(&header, Some(c), &key);
    return token;
}

pub fn verify_jwt<T: JwtClaims>(token: &str) -> Result<T, jsonwebtoken::errors::Error> {
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be configured on .env");
    let key = DecodingKey::from_secret(secret.as_bytes());

    decode::<T>(
        token,
        &key,
        &Validation::new(jsonwebtoken::Algorithm::HS256),
    )
    .map(|d| d.claims)
}
