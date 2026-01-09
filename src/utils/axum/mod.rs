/*******************************************************************************
 * Copyright (c) 2024 Cénotélie Opérations SAS (cenotelie.fr)
******************************************************************************/

//! Utility APIs for axum

pub mod auth;
pub mod embedded;
pub mod extractors;
pub mod sse;

use std::{
    backtrace::{Backtrace, BacktraceStatus},
    error::Error,
    fmt::Write as _,
};

use axum::Json;
use axum::http::StatusCode;
use log::{error, info};
use uuid::Uuid;

use crate::utils::apierror::{ApiError, AsStatusCode, ResponseError};

/// Defines an API response
pub type ApiResult<T> = Result<(StatusCode, Json<T>), (StatusCode, Json<ResponseError>)>;

/// Produces an error response
pub fn response_error_http(http: StatusCode, error: ApiError) -> (StatusCode, Json<ResponseError>) {
    let uuid = Uuid::new_v4();
    if http == StatusCode::INTERNAL_SERVER_ERROR {
        // log internal errors
        error!("{uuid} ApiError {error}");
        if let Some(backtrace) = &error.backtrace {
            error!("{backtrace}");
        }
    } else {
        info!("{uuid} ApiError {error}");
    }
    let body = Json(ResponseError::new(uuid, error.message, error.details));
    (http, body)
}

/// Produces an error response
pub fn response_error(error: ApiError) -> (StatusCode, Json<ResponseError>) {
    response_error_http(error.http, error)
}

/// Produces an error response
pub fn into_response_error(error: &(impl AsStatusCode + Send + Sync + 'static)) -> (StatusCode, Json<ResponseError>) {
    let status_code = error.status_code();
    let uuid = Uuid::new_v4();
    let log_msg = display_error(uuid, error);
    if status_code == StatusCode::INTERNAL_SERVER_ERROR {
        // log internal errors
        error!("{log_msg}");
        let backtrace = Backtrace::capture();
        if backtrace.status() == BacktraceStatus::Captured {
            error!("{backtrace}");
        }
    } else {
        info!("{log_msg}");
    }
    let body = Json(ResponseError::new(uuid, error.to_string(), None));
    (status_code, body)
}

pub fn display_error(uuid: Uuid, error: &impl Error) -> String {
    let sources = ErrorSourcesIter::from(error.source());
    sources.enumerate().fold(String::new(), |mut msg, (idx, error)| {
        if idx == 0 {
            let _ = write!(&mut msg, "{uuid} {error}");
        } else {
            let _ = write!(&mut msg, "\n\t[{idx}] {error}");
        }
        msg
    })
}

#[derive(Clone, Debug)]
pub struct ErrorSourcesIter<'a>(Option<&'a (dyn Error + 'static)>);

impl<'a> From<&'a (dyn Error + 'static)> for ErrorSourcesIter<'a> {
    fn from(err: &'a (dyn Error + 'static)) -> Self {
        ErrorSourcesIter(Some(err))
    }
}
impl<'a> From<Option<&'a (dyn Error + 'static)>> for ErrorSourcesIter<'a> {
    fn from(err: Option<&'a (dyn Error + 'static)>) -> Self {
        ErrorSourcesIter(err)
    }
}

impl<'a> Iterator for ErrorSourcesIter<'a> {
    type Item = &'a (dyn Error + 'static);

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .map_or_default(|err| err.source())
            .and_then(|next| self.0.replace(next))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.0.is_some() { (1, None) } else { (0, Some(0)) }
    }
}

impl std::iter::FusedIterator for ErrorSourcesIter<'_> {}

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
