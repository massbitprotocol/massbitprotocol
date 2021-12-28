use crate::user_managerment::{error::Error, Result, WebResult};
use chrono::prelude::*;
use hex;
use jsonwebtoken::{
    dangerous_insecure_decode, dangerous_insecure_decode_with_validation, decode, decode_header,
    encode, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;
use warp::{
    filters::header::headers_cloned,
    http::header::{HeaderMap, HeaderValue, AUTHORIZATION},
    reject, Filter, Rejection,
};

lazy_static! {
    pub static ref BEARER: String = String::from("Bearer ");
    pub static ref JWT_SECRET_KEY: String = env::var("JWT_SECRET_KEY").unwrap_or(String::from(
        "1F9853B9E546038E0736CB32C97085E07868552078A7745FBA3C5C78889F4CD2"
    ));
}
#[derive(Clone, PartialEq)]
pub enum Role {
    User,
    //Admin,
}

impl Role {
    pub fn from_str(role: &str) -> Role {
        match role {
            //"Admin" => Role::Admin,
            _ => Role::User,
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::User => write!(f, "User"),
            //Role::Admin => write!(f, "Admin"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Claims {
    sub: String,
    exp: usize,
    iat: usize,
    iss: String,
}

pub fn with_auth(role: Role) -> impl Filter<Extract = (String,), Error = Rejection> + Clone {
    headers_cloned()
        .map(move |headers: HeaderMap<HeaderValue>| (role.clone(), headers))
        .and_then(authorize)
}

pub fn create_jwt(uid: &str, role: &Role) -> Result<String> {
    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::seconds(60))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: uid.to_owned(),
        iat: Utc::now().timestamp() as usize,
        exp: expiration as usize,
        iss: "Massbit".to_string(),
    };
    let header = Header::new(Algorithm::HS256);
    encode(
        &header,
        &claims,
        &EncodingKey::from_base64_secret(&JWT_SECRET_KEY).unwrap(),
    )
    .map_err(|_| Error::JWTTokenCreationError)
}

async fn authorize((role, headers): (Role, HeaderMap<HeaderValue>)) -> WebResult<String> {
    match jwt_from_header(&headers) {
        Ok(jwt) => {
            let decoded = decode::<Claims>(
                &jwt,
                &DecodingKey::from_rsa_pem(include_bytes!("pubkey.pem"))
                    .expect("Invalid pubkey in pubkey.pem"),
                &Validation::new(Algorithm::RS256),
            )
            .map_err(|_| reject::custom(Error::JWTTokenError))?;

            info!("authorize: {:?}", decoded.claims);
            Ok(decoded.claims.sub)
        }
        Err(e) => return Err(reject::custom(e)),
    }
}

fn jwt_from_header(headers: &HeaderMap<HeaderValue>) -> Result<String> {
    let header = match headers.get(AUTHORIZATION) {
        Some(v) => v,
        None => return Err(Error::NoAuthHeaderError),
    };
    let auth_header = match std::str::from_utf8(header.as_bytes()) {
        Ok(v) => v,
        Err(_) => return Err(Error::NoAuthHeaderError),
    };
    if !auth_header.starts_with(BEARER.as_str()) {
        return Err(Error::InvalidAuthHeaderError);
    }
    Ok(auth_header.trim_start_matches(BEARER.as_str()).to_owned())
}
