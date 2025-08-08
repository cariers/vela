pub use jsonwebtoken::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims<T> {
    pub jti: String,
    pub aud: String,
    pub exp: u64,
    pub iat: u64,
    pub iss: String,
    pub sub: String,
    #[serde(flatten)]
    pub payload: T,
}
