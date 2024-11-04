use core::fmt;

use axum::{http::StatusCode, response::IntoResponse};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    Database(sqlx::Error),
    Write,
    Read,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Write => (StatusCode::INTERNAL_SERVER_ERROR, "fail to write").into_response(),
            Self::Read => (StatusCode::INTERNAL_SERVER_ERROR, "fail to read").into_response(),
            Self::Database(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)).into_response()
            }
        }
    }
}
