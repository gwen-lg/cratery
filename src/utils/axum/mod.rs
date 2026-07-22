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
pub fn into_response_error(error: impl AsStatusCode + Send + Sync + 'static) -> (StatusCode, Json<ResponseError>) {
    let status_code = error.status_code();
    // let error = anyhow::Error::from(error);
    let uuid = Uuid::new_v4();
    let log_msg = display_error(uuid, &error);
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

pub fn display_error(uuid: Uuid, error: impl Error) -> String {
    let error_msg = format!("{uuid} {error}");
    let sources = error.source();
    sources.iter().fold(error_msg, |mut msg, error| {
        //TODO: use fold
        msg.push('\n');
        msg.push_str(&error.to_string());
        msg
    })
}

#[derive(Clone, Debug)]
pub struct Source<'a> {
    current: Option<&'a (dyn Error + 'static)>,
}

impl<'a> Source<'a> {
    pub fn new(source: Option<&'a (dyn Error + 'static)>) -> Self {
        Self { current: source }
    }
}

impl<'a> Iterator for Source<'a> {
    type Item = &'a (dyn Error + 'static);

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current;
        self.current = self.current.and_then(Error::source);
        current
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.current.is_some() { (1, None) } else { (0, Some(0)) }
    }
}

impl<'a> std::iter::FusedIterator for Source<'a> {}

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
