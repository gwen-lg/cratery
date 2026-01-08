/*******************************************************************************
 * Copyright (c) 2024 Cénotélie Opérations SAS (cenotelie.fr)
 ******************************************************************************/

//! Custom errors

use std::env::VarError;
use std::fmt::Display;

/// Error when an environment error is missing
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MissingEnvVar {
    /// The original error
    pub(crate) original: VarError,
    /// The name of the variable
    pub(crate) var_name: String,
}

impl Display for MissingEnvVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "missing expected env var {}", self.var_name)
    }
}

impl std::error::Error for MissingEnvVar {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.original)
    }
}
