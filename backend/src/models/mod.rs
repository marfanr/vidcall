pub mod conn;
pub mod refresh_tokens;
pub mod room;
pub mod user;

pub trait StandardModel {
    fn save();
}
