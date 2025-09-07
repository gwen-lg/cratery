/*******************************************************************************
 * Copyright (c) 2024 Cénotélie Opérations SAS (cenotelie.fr)
 ******************************************************************************/

//! Service for persisting information in the database
//! API related to administration of the registry itself

use chrono::Local;
use thiserror::Error;

use super::Database;
use crate::model::auth::{RegistryUserToken, RegistryUserTokenWithSecret};
use crate::utils::token::{generate_token, hash_token};

#[derive(Debug, Error)]
pub enum TokensError {
    ///TODO: replace with dedicated error
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error("a token with the same name already exists")]
    TokenNameExist, //error_invalid_request(),
}

impl Database {
    /// Gets the global tokens for the registry, usually for CI purposes
    pub async fn get_global_tokens(&self) -> Result<Vec<RegistryUserToken>, sqlx::Error> {
        let rows = sqlx::query!("SELECT id, name, lastUsed AS last_used FROM RegistryGlobalToken ORDER BY id",)
            .fetch_all(&mut *self.transaction.borrow().await)
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| RegistryUserToken {
                id: row.id,
                name: row.name,
                last_used: row.last_used,
                can_write: false,
                can_admin: false,
            })
            .collect())
    }

    /// Creates a global token for the registry
    pub async fn create_global_token(&self, name: &str) -> Result<RegistryUserTokenWithSecret, TokensError> {
        let row = sqlx::query!("SELECT id FROM RegistryGlobalToken WHERE name = $1 LIMIT 1", name)
            .fetch_optional(&mut *self.transaction.borrow().await)
            .await?;
        if row.is_some() {
            return Err(TokensError::TokenNameExist);
        }
        let token_secret = generate_token(64);
        let token_hash = hash_token(&token_secret);
        let now = Local::now().naive_local();
        let id = sqlx::query!(
            "INSERT INTO RegistryGlobalToken (name, token, lastUsed) VALUES ($1, $2, $3) RETURNING id",
            name,
            token_hash,
            now,
        )
        .fetch_one(&mut *self.transaction.borrow().await)
        .await?
        .id;
        Ok(RegistryUserTokenWithSecret {
            id,
            name: name.to_string(),
            secret: token_secret,
            last_used: now,
            can_write: false,
            can_admin: false,
        })
    }

    /// Revokes a global token for the registry
    pub async fn revoke_global_token(&self, token_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM RegistryGlobalToken WHERE id = $1", token_id)
            .execute(&mut *self.transaction.borrow().await)
            .await?;
        Ok(())
    }
}
