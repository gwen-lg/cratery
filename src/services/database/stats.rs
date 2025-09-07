/*******************************************************************************
 * Copyright (c) 2024 Cénotélie Opérations SAS (cenotelie.fr)
 ******************************************************************************/

//! Service for persisting information in the database
//! API related to statistics

use thiserror::Error;

use super::Database;
use crate::model::CrateVersion;
use crate::model::stats::GlobalStats;

#[derive(Debug, Error)]
pub enum CratesStatsError {
    #[error("error during execution of request to count crates")]
    CountCrates(#[source] sqlx::Error),
    #[error("error during execution of request to count downloads")]
    CountDownload(#[source] sqlx::Error),

    #[error("error during execution of request to get info of `newest` crates")]
    NewestCrates(#[source] sqlx::Error),
    #[error("error during execution of request to get info of `most downloaded` crates")]
    MostDownloaded(#[source] sqlx::Error),
    #[error("error during execution of request to get info of `last updated` crates")]
    LastUpdated(#[source] sqlx::Error),
}

impl Database {
    /// Gets the global statistics for the registry
    pub async fn get_crates_stats(&self) -> Result<GlobalStats, CratesStatsError> {
        let total_crates = sqlx::query!("SELECT COUNT(name) AS total_crates FROM Package")
            .fetch_one(&mut *self.transaction.borrow().await)
            .await
            .map_err(CratesStatsError::CountCrates)?
            .total_crates;
        let total_downloads = sqlx::query!("SELECT SUM(downloadCount) AS total_downloads FROM PackageVersion")
            .fetch_one(&mut *self.transaction.borrow().await)
            .await
            .map_err(CratesStatsError::CountDownload)?
            .total_downloads
            .unwrap_or_default();

        let rows = sqlx::query!(
            "SELECT name, version, upload
            FROM Package INNER JOIN PackageVersion ON package = name
            WHERE (SELECT COUNT(version) FROM PackageVersion WHERE package = name) = 1
            ORDER BY upload DESC
            LIMIT 10"
        )
        .fetch_all(&mut *self.transaction.borrow().await)
        .await
        .map_err(CratesStatsError::NewestCrates)?;
        let crates_newest = rows
            .into_iter()
            .map(|row| CrateVersion {
                package: row.name,
                version: row.version,
            })
            .collect::<Vec<_>>();

        let rows = sqlx::query!(
            "SELECT name, SUM(downloadCount) AS download_count
            FROM Package INNER JOIN PackageVersion ON package = name
            GROUP BY package
            ORDER BY download_count DESC
            LIMIT 10"
        )
        .fetch_all(&mut *self.transaction.borrow().await)
        .await
        .map_err(CratesStatsError::MostDownloaded)?;
        let crates_most_downloaded = rows
            .into_iter()
            .map(|row| CrateVersion {
                package: row.name,
                version: String::new(),
            })
            .collect::<Vec<_>>();

        let rows = sqlx::query!(
            "SELECT package, version, upload
            FROM PackageVersion
            ORDER BY upload DESC
            LIMIT 10"
        )
        .fetch_all(&mut *self.transaction.borrow().await)
        .await
        .map_err(CratesStatsError::LastUpdated)?;
        let crates_last_updated = rows
            .into_iter()
            .map(|row| CrateVersion {
                package: row.package,
                version: row.version,
            })
            .collect::<Vec<_>>();

        Ok(GlobalStats {
            total_downloads,
            total_crates,
            crates_newest,
            crates_most_downloaded,
            crates_last_updated,
        })
    }
}
