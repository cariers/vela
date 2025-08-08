use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{
    ids::{PlayerId, SessionId},
    jwt,
};

#[async_trait::async_trait]
pub trait Authenticator<T> {
    async fn authenticate(&self, token: T) -> Result<(SessionId, PlayerId), AuthError>;
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Empty token provided")]
    EmptyToken,
    #[error("Invalid token format: {0}")]
    InvalidToken(String),
    #[error("Token has expired")]
    ExpiredToken,
    #[error("Unauthorized access")]
    Unauthorized,
    #[error("Internal server error")]
    Io(#[from] std::io::Error),
    #[error("Authentication service unavailable")]
    Timeout,
}

#[derive(Clone)]
pub struct JwtAuthenticator {
    secret: jwt::DecodingKey,
    validation: jwt::Validation,
}

impl JwtAuthenticator {
    pub fn new(secret: jwt::DecodingKey) -> Self {
        Self {
            secret,
            validation: jwt::Validation::default(),
        }
    }

    pub fn with_validation(mut self, validation: jwt::Validation) -> Self {
        self.validation = validation;
        self
    }

    pub fn validation_mut(&mut self) -> &mut jwt::Validation {
        &mut self.validation
    }
}

#[async_trait::async_trait]
impl Authenticator<String> for JwtAuthenticator {
    async fn authenticate(&self, token: String) -> Result<(SessionId, PlayerId), AuthError> {
        let claims = jwt::decode::<Claims<()>>(token.as_ref(), &self.secret, &self.validation)
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?
            .claims;
        Ok((claims.jti, claims.sub))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Claims<T> {
    pub jti: SessionId,           // Session ID
    pub aud: Option<Vec<String>>, // 授权的范围
    pub exp: u64,
    pub iat: u64,
    pub iss: Option<String>, // 颁发令牌的服务的名称
    pub sub: PlayerId,       // 用户Id
    #[serde(flatten)]
    pub payload: T,
}
