/*******************************************************************************
 * Copyright (c) 2024 Cénotélie Opérations SAS (cenotelie.fr)
 ******************************************************************************/

//! Definition of the error type for API requests

use std::backtrace::Backtrace;
use std::fmt::{Display, Formatter};

use serde_derive::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug)]
struct SourceError(anyhow::Error);

impl<E> From<E> for SourceError
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(error: E) -> Self {
        Self(anyhow::Error::from(error))
    }
}

/// Describes an API error
#[derive(Serialize, Deserialize, Debug)]
pub struct ApiError {
    /// The associated HTTP error code
    pub http: u16,
    /// A custom error message
    pub message: String,
    /// Optional details for the error
    pub details: Option<String>,
    /// Optional Error source
    #[serde(skip_serializing, skip_deserializing)] // We don't want  source send to client
    source: Option<SourceError>,
    /// The backtrace when the error was produced
    #[serde(skip_serializing, skip_deserializing)]
    pub backtrace: Option<Backtrace>,
}

impl ApiError {
    /// Creates a new error
    #[allow(clippy::needless_pass_by_value)]
    #[must_use]
    pub fn new<M: ToString>(http: u16, message: M, details: Option<String>) -> Self {
        Self {
            http,
            message: message.to_string(),
            details,
            source: None,
            backtrace: Some(Backtrace::capture()),
        }
    }
}

//TODO: separate to client and to log Display
impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let details = self.details.as_ref().map_or("", std::convert::AsRef::as_ref);
        write!(f, "{} ({})", &self.message, &details)?;
        if let Some(source) = self.source.as_ref() {
            writeln!(f)?;
            //writeln!(f, "\t {}", source.0)
            source
                .0
                .chain()
                .enumerate()
                .try_for_each(|(idx, err)| writeln!(f, "\t [{idx}] {err}"))?;
        }
        Ok(())
    }
}

impl Clone for ApiError {
    fn clone(&self) -> Self {
        Self {
            http: self.http,
            message: self.message.clone(),
            details: self.details.clone(),
            source: None, //This is bad
            backtrace: None,
        }
    }
}

impl<E> From<E> for ApiError
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: E) -> Self {
        Self {
            http: 500,
            message: "TODO: Look parent".into(),
            details: None,
            source: Some(err.into()),
            backtrace: Some(Backtrace::capture()),
        }
        //Self::new(500, "The operation failed in the backend.", Some(err.to_string()))
    }
}

/// Specializes an API error with additional details
pub fn specialize(original: ApiError, details: String) -> ApiError {
    ApiError {
        details: Some(details),
        ..original
    }
}

/// Error when the operation failed in the backend
#[must_use]
pub fn error_backend_failure() -> ApiError {
    ApiError::new(500, "The operation failed in the backend.", None)
}

/// Error when the operation failed due to invalid input
#[must_use]
pub fn error_invalid_request() -> ApiError {
    ApiError::new(400, "The request could not be understood by the server.", None)
}

/// Error when the user is not authorized (not logged in)
#[must_use]
pub fn error_unauthorized() -> ApiError {
    ApiError::new(401, "User is not authenticated.", None)
}

/// Error when the requested action is forbidden to the (otherwise authenticated) user
#[must_use]
pub fn error_forbidden() -> ApiError {
    ApiError::new(403, "This action is forbidden to the user.", None)
}

/// Error when the requested user cannot be found
#[must_use]
pub fn error_not_found() -> ApiError {
    ApiError::new(404, "The requested resource cannot be found.", None)
}

/// Error when the request has a conflicts
#[must_use]
pub fn error_conflict() -> ApiError {
    ApiError::new(
        408,
        "The request could not be processed because of conflict in the current state of the resource.",
        None,
    )
}

/// A helper to help remove of [`ApiError`] where it's not appropriated.
#[derive(Debug, Error)]
pub struct UnApiError {
    message: String,
    details: Option<String>,
}

impl Display for UnApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "UnApiError : {}", self.message)?;
        if let Some(details) = &self.details {
            writeln!(f, "\t {details}")?;
        }
        Ok(())
    }
}

impl From<ApiError> for UnApiError {
    fn from(value: ApiError) -> Self {
        Self {
            message: value.message,
            details: value.details,
        }
    }
}
