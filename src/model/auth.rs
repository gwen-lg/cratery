/*******************************************************************************
 * Copyright (c) 2024 Cénotélie Opérations SAS (cenotelie.fr)
 ******************************************************************************/

//! Objects related to authentication

use chrono::NaiveDateTime;
use serde_derive::{Deserialize, Serialize};

use crate::utils::apierror::{ApiError, error_forbidden, error_invalid_request, specialize};

/// The admin role
pub(crate) const ROLE_ADMIN: &str = "admin";

/// Represents a data about a successful authentication
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Authentication {
    /// The principal (email of the user)
    pub(crate) principal: AuthenticationPrincipal,
    /// Whether a crate can be uploaded
    #[serde(rename = "canWrite")]
    pub(crate) can_write: bool,
    /// Whether administration can be done
    #[serde(rename = "canAdmin")]
    pub(crate) can_admin: bool,
}

impl Authentication {
    /// Creates a new authentication for a self connection
    #[must_use]
    pub(crate) const fn new_self() -> Self {
        Self {
            principal: AuthenticationPrincipal::SelfAuth,
            can_write: false,
            can_admin: false,
        }
    }

    // Creates a new authentication for a service using a global token
    #[must_use]
    pub(crate) const fn new_service(token_id: String) -> Self {
        Self {
            principal: AuthenticationPrincipal::Service { token_id },
            can_write: false,
            can_admin: false,
        }
    }

    // Creates a new user authentication that can do everything
    #[must_use]
    pub(crate) const fn new_user(uid: i64, email: String) -> Self {
        Self {
            principal: AuthenticationPrincipal::User { uid, email },
            can_write: true,
            can_admin: true,
        }
    }

    /// Gets the uid of the associated user
    pub(crate) fn uid(&self) -> Result<i64, ApiError> {
        if let AuthenticationPrincipal::User { uid, email: _ } = &self.principal {
            Ok(*uid)
        } else {
            Err(specialize(
                error_invalid_request(),
                String::from("Expected a user to be authenticated"),
            ))
        }
    }

    /// Gets the email of the associated user
    pub(crate) fn email(&self) -> Result<&str, ApiError> {
        if let AuthenticationPrincipal::User { uid: _, email } = &self.principal {
            Ok(email)
        } else {
            Err(specialize(
                error_invalid_request(),
                String::from("Expected a user to be authenticated"),
            ))
        }
    }

    /// Checks that this authentication enables writing
    pub(crate) fn check_can_write(&self) -> Result<(), ApiError> {
        if self.can_write {
            Ok(())
        } else {
            Err(specialize(
                error_forbidden(),
                String::from("writing is forbidden for this authentication"),
            ))
        }
    }

    /// Checks that this authentication enables admin tasks
    pub(crate) fn check_can_admin(&self) -> Result<(), ApiError> {
        if self.can_admin {
            Ok(())
        } else {
            Err(specialize(
                error_forbidden(),
                String::from("administration is forbidden for this authentication"),
            ))
        }
    }
}

/// The principal associated to an authentication
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) enum AuthenticationPrincipal {
    /// A user is authenticated
    User { uid: i64, email: String },
    /// A service through a global token
    Service { token_id: String },
    /// The registry itself when connecting to itself
    SelfAuth,
}

/// A token for a registry user
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct RegistryUserToken {
    /// The unique identifier
    pub(crate) id: i64,
    /// The token name
    pub(crate) name: String,
    /// The last time the token was used
    #[serde(rename = "lastUsed")]
    pub(crate) last_used: NaiveDateTime,
    /// Whether a crate can be uploaded using this token
    #[serde(rename = "canWrite")]
    pub(crate) can_write: bool,
    /// Whether administration can be done using this token through the API
    #[serde(rename = "canAdmin")]
    pub(crate) can_admin: bool,
}

/// A token for a registry user
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct RegistryUserTokenWithSecret {
    /// The unique identifier
    pub(crate) id: i64,
    /// The token name
    pub(crate) name: String,
    /// The value for the token
    pub(crate) secret: String,
    /// The last time the token was used
    #[serde(rename = "lastUsed")]
    pub(crate) last_used: NaiveDateTime,
    /// Whether a crate can be uploaded using this token
    #[serde(rename = "canWrite")]
    pub(crate) can_write: bool,
    /// Whether administration can be done using this token through the API
    #[serde(rename = "canAdmin")]
    pub(crate) can_admin: bool,
}

/// An OAuth access token
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct OAuthToken {
    /// The access token
    pub(crate) access_token: String,
    /// The type of token
    pub(crate) token_type: String,
    /// The expiration time
    pub(crate) expires_in: Option<i64>,
    /// The refresh token
    pub(crate) refresh_token: Option<String>,
    /// The grant scope
    pub(crate) scope: Option<String>,
}

/// Finds a field in a JSON blob
#[must_use]
pub(crate) fn find_field_in_blob<'v>(blob: &'v serde_json::Value, path: &str) -> Option<&'v str> {
    let mut last = blob;
    for item in path.split('.') {
        last = last.as_object()?.get(item)?;
    }
    last.as_str()
}

/// The kind of auth token
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenKind {
    /// A user-specific token
    User,
    /// A registry-wide token
    Registry,
}

/// Event when a token was used
#[derive(Debug, Clone)]
pub(crate) struct TokenUsage {
    /// The kind of token
    pub(crate) kind: TokenKind,
    /// The unique identifier for the token
    pub(crate) token_id: i64,
    /// The timestamp when the token was used
    pub(crate) timestamp: NaiveDateTime,
}
