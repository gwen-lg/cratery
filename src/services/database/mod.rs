/*******************************************************************************
 * Copyright (c) 2024 Cénotélie Opérations SAS (cenotelie.fr)
 ******************************************************************************/

//! Service for persisting information in the database

pub mod admin;
pub mod jobs;
pub mod packages;
pub mod stats;
pub mod users;

use packages::CratesError;
use std::future::Future;
use thiserror::Error;

use crate::application::AuthenticationError;
use crate::model::auth::ROLE_ADMIN;
use crate::utils::db::{AppTransaction, RwSqlitePool};

//TODO: document, en move earlier in file
#[derive(Debug, Error)]
pub enum DbReadError {
    #[error("Failed to acquire read")]
    AcquireRead {
        #[source]
        source: sqlx::Error,
    },

    #[error("Error in workload")]
    Workload {
        #[source]
        source: anyhow::Error,
    },

    #[error("Failed to commit")]
    Commit(#[source] sqlx::Error),
    #[error("Failed to rollback after error :\n{error}")]
    Rollback {
        #[source]
        source: sqlx::Error,
        error: String, //TODO: manage this better ?
    },
}

/// Executes a piece of work in the context of a transaction
/// The transaction is committed if the operation succeed,
/// or rolled back if it fails
///
/// # Errors
///
/// Returns an instance of the `E` type argument
pub async fn db_transaction_read<F, FUT, T, E>(pool: &RwSqlitePool, workload: F) -> Result<T, DbReadError>
where
    F: FnOnce(Database) -> FUT,
    FUT: Future<Output = Result<T, E>>,
    E: Into<anyhow::Error>,
{
    let transaction = pool
        .acquire_read()
        .await
        .map_err(|source| DbReadError::AcquireRead { source })?;
    let result = {
        let database = Database {
            transaction: transaction.clone(),
        };
        workload(database).await
    };
    let transaction = transaction.into_original().unwrap();
    match result {
        Ok(t) => {
            transaction.commit().await.map_err(DbReadError::Commit)?;
            Ok(t)
        }
        Err(error) => {
            let workload_err = error.into();
            transaction.rollback().await.map_err(|source| DbReadError::Rollback {
                source,
                error: workload_err.to_string(),
            })?;
            Err(DbReadError::Workload { source: workload_err })
        }
    }
}

//TODO: document, en move earlier in file
#[derive(Debug, Error)]
pub enum DbWriteError {
    #[error("Failed to acquire write for operation `{operation}`")]
    AcquireWrite {
        #[source]
        source: sqlx::Error,
        operation: &'static str,
    },

    #[error("Error in workload for operation `{operation}`")]
    Workload {
        #[source]
        source: anyhow::Error,
        operation: &'static str,
    },

    #[error("Failed to commit operation `{operation}`")]
    Commit {
        #[source]
        source: sqlx::Error,
        operation: &'static str,
    },

    #[error("Failed to rollback operation `{operation}` after error :\n{error}")]
    Rollback {
        #[source]
        source: sqlx::Error,
        operation: &'static str,
        error: String, //TODO: manage this better ?
    },
}

/// Executes a piece of work in the context of a transaction
/// The transaction is committed if the operation succeed,
/// or rolled back if it fails
///
/// # Errors
///
/// Returns an instance of the `E` type argument
pub async fn db_transaction_write<F, FUT, T, E>(
    pool: &RwSqlitePool,
    operation: &'static str,
    workload: F,
) -> Result<T, DbWriteError>
where
    F: FnOnce(Database) -> FUT,
    FUT: Future<Output = Result<T, E>>,
    E: Into<anyhow::Error>, //std::error::Error + std::marker::Send + std::marker::Sync + 'static,
{
    let transaction = pool
        .acquire_write(operation)
        .await
        .map_err(|source| DbWriteError::AcquireWrite { source, operation })?;
    let result = {
        let database = Database {
            transaction: transaction.clone(),
        };
        workload(database).await
    };
    let transaction = transaction.into_original().unwrap();
    match result {
        Ok(t) => {
            transaction
                .commit()
                .await
                .map_err(|source| DbWriteError::Commit { source, operation })?;
            Ok(t)
        }
        Err(error) => {
            let workload_err = error.into();
            transaction.rollback().await.map_err(|source| DbWriteError::Rollback {
                source,
                operation,
                error: workload_err.to_string(),
            })?;
            Err(DbWriteError::Workload {
                source: workload_err,
                operation,
            })
        }
    }
}

//TODO: use ApiErrorNext ?
// conflict with From<T> for ApiError
// impl Into<ApiError> for DbWriteError {
//     fn into(self) -> ApiError {
//         //TODO: write info to log with uuid and print uuid in error to write to client
//         match self {
//             DbWriteError::AcquireWrite { .. } => ApiError::new(500, "TODO: write info to log", None),
//             DbWriteError::Workload { source, operation } => {
//                 ApiError::new(500, format!("operation `{operation}` failed with : {source}"))
//             } //TODO: keep information for http from workload
//             DbWriteError::Commit { source, operation } => ApiError::new(500, "TODO: write info to log", None),
//             DbWriteError::Rollback {
//                 source,
//                 operation,
//                 error,
//             } => ApiError::new(500, "TODO: write info to log", None),
//         }
//     }
// }

/// Represents the application
pub struct Database {
    /// The connection
    pub(crate) transaction: AppTransaction,
}

impl Database {
    /// Checks the security for an operation and returns the identifier of the target user (login)
    pub async fn check_is_user(&self, email: &str) -> Result<i64, AuthenticationError> {
        let maybe_row = sqlx::query!("SELECT id FROM RegistryUser WHERE isActive = TRUE AND email = $1", email)
            .fetch_optional(&mut *self.transaction.borrow().await)
            .await
            .map_err(AuthenticationError::CheckUser)?;
        let row = maybe_row.ok_or(AuthenticationError::Unauthorized)?;
        Ok(row.id)
    }

    /// Checks that a user is an admin
    pub async fn get_is_admin(&self, uid: i64) -> Result<bool, AuthenticationError> {
        let roles = sqlx::query!("SELECT roles FROM RegistryUser WHERE id = $1", uid)
            .fetch_optional(&mut *self.transaction.borrow().await)
            .await
            .map_err(AuthenticationError::CheckRoles)?
            .ok_or(AuthenticationError::Forbidden)?
            .roles;
        Ok(roles.split(',').any(|role| role.trim() == ROLE_ADMIN))
    }

    /// Checks that a user is an admin
    pub async fn check_is_admin(&self, uid: i64) -> Result<(), AuthenticationError> {
        let is_admin = self.get_is_admin(uid).await?;
        if is_admin {
            Ok(())
        } else {
            Err(AuthenticationError::AdministrationIsForbidden)
        }
    }

    /// Checks that a package exists
    /// TODO: why it's not in packages.rs ?
    pub async fn check_crate_exists(&self, package: &str, version: &str) -> Result<(), CratesError> {
        let _row = sqlx::query!(
            "SELECT id FROM PackageVersion WHERE package = $1 AND version = $2 LIMIT 1",
            package,
            version
        )
        .fetch_optional(&mut *self.transaction.borrow().await)
        .await? //TODO: add dedicated error ?
        .ok_or_else(|| CratesError::PackageVersionNotFound {
            package: package.into(),
            version: version.into(),
        })?;
        Ok(())
    }

    /// Checks the ownership of a package
    pub async fn check_is_crate_manager(&self, uid: i64, package: &str) -> Result<i64, IsCrateManagerError> {
        if self.check_is_admin(uid).await.is_ok() {
            return Ok(uid);
        }
        let row = sqlx::query!(
            "SELECT id from PackageOwner WHERE package = $1 AND owner = $2 LIMIT 1",
            package,
            uid
        )
        .fetch_optional(&mut *self.transaction.borrow().await)
        .await?;
        match row {
            Some(_) => Ok(uid),
            None => Err(IsCrateManagerError::NotOwnerOfPackage),
        }
    }
}

///TODO: documentation
#[derive(Debug, Error)]
pub enum IsCrateManagerError {
    #[error("Failed to execute db request.")]
    Sqlx(#[from] sqlx::Error),

    #[error("User is not an owner of this package")]
    NotOwnerOfPackage,
    //  specialize(
    //     error_forbidden(),
    //     String::from("User is not an owner of this package"),
    // )
}
