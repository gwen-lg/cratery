/*******************************************************************************
 * Copyright (c) 2024 Cénotélie Opérations SAS (cenotelie.fr)
******************************************************************************/

//! Utility APIs for axum

pub mod auth;
pub mod embedded;
pub mod extractors;
pub mod sse;

use axum::Json;
use axum::http::StatusCode;
use log::error;
use uuid::Uuid;

use crate::utils::apierror::{ApiError, ResponseError};

/// Defines an API response
pub type ApiResult<T> = Result<(StatusCode, Json<T>), (StatusCode, Json<ResponseError>)>;

/// Produces an error response
pub fn response_error_http(http: StatusCode, error: ApiError) -> (StatusCode, Json<ResponseError>) {
    let uuid = Uuid::new_v4();
    if http == StatusCode::INTERNAL_SERVER_ERROR {
        // log internal errors
        error!("{uuid} {error:?}");
        if let Some(backtrace) = &error.backtrace {
            error!("{backtrace}");
        }
    }
    let body = Json(ResponseError::new(uuid, error.message, error.details));
    (http, body)
}

/// Produces an error response
pub fn response_error(error: ApiError) -> (StatusCode, Json<ResponseError>) {
    response_error_http(error.http, error)
}

/// Produces an OK response
pub const fn response_ok<T>(data: T) -> (StatusCode, Json<T>) {
    (StatusCode::OK, Json(data))
}

/// Maps a service result to a web api result
///
/// # Errors
///
/// Maps the corresponding error from the given `Result`.
pub fn response<T>(result: Result<T, ApiError>) -> ApiResult<T> {
    result.map_err(response_error).map(response_ok)
}
