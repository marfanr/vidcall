use argon2::{Algorithm, Argon2, Params, Version};

pub fn get_argon<'a>() -> Argon2<'a> {
    Argon2::new_with_secret(
        b"haiya",
        Algorithm::Argon2id,
        Version::V0x13,
        Params::default(),
    )
    .expect("error to setup argon")
}
