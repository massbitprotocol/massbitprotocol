use warp::Rejection;

type Result<T> = std::result::Result<T, error::Error>;
type WebResult<T> = std::result::Result<T, Rejection>;

pub mod auth;
pub mod error;
